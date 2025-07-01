use std::{
    array,
    fmt::Display,
    num::NonZeroU8,
    ops::{Index, IndexMut},
    str::FromStr,
};

use super::{
    argument::Argument,
    computer::{RegisterAccessError, RegisterMap, RegisterSet},
    integer::{AssignIntegerError, BiggerInteger, Integer},
};

pub type ArgumentValues = [Option<Integer>; Instruction::NUM_ARGUMENTS];

#[derive(Clone, Copy, Debug)]
pub struct Instruction {
    pub kind: InstructionKind,
    pub line: u32,
    pub arguments: [Argument; Self::NUM_ARGUMENTS],
}

impl Instruction {
    pub const NUM_ARGUMENTS: usize = 3;

    /// Returns the total time taken by the instruction, the value of each input, and whether or
    /// not to update `previous_instruction`.
    pub fn evaluate(
        &self,
        registers: &mut RegisterSet,
        previous_instruction: Option<(&Instruction, &ArgumentValues)>,
        next_instruction: &mut u32,
        runtime: u64,
    ) -> Result<(u32, ArgumentValues, bool), InstructionEvaluationInterrupt> {
        let properties = self.kind.get_properties();

        let mut total_time = 0;

        let mut argument_values = [None; 3];

        let mut registers_read = RegisterMap::from_element(0u8);

        for (i, value) in argument_values.iter_mut().enumerate() {
            let requirement = properties.arguments[i];

            assert!(self.arguments[i].matches_requirement(requirement));

            if matches!(requirement, ArgumentRequirement::RegisterWriteOnly) {
                continue;
            }

            *value = match self.arguments[i] {
                Argument::Instruction(_) => None,
                Argument::Number(source) => {
                    let (value, register) = source.value(registers)?;

                    if let Some(register) = register {
                        registers_read[register as usize] += 1;
                    }
                    Some(value)
                }
                Argument::Comparison(comparison) => {
                    let (value, registers) = comparison.evaluate(registers)?;

                    for register in registers.into_iter().flatten() {
                        registers_read[register as usize] += 1;
                    }
                    Some(value)
                }
                Argument::Empty => None,
            };
        }

        let mut read_time = 0;

        for (register, num_reads) in registers_read.into_iter().enumerate() {
            if num_reads > 0 {
                read_time = read_time.max(registers.get(register as u32).unwrap().read_time);
            }
        }

        total_time += read_time;

        let (instruction_time, update_previous_instruction) =
            self.execution_time(previous_instruction, &argument_values);

        total_time += instruction_time;

        let mut jump = None;

        match self.kind {
            InstructionKind::Set => {
                total_time += self.write_to_argument(registers, 0, argument_values[1].unwrap())?;
            }
            InstructionKind::Add => {
                total_time += self.apply_operation(
                    argument_values,
                    registers,
                    [0, 1, 2],
                    |a, b| a.checked_add(b),
                    |a, b| a + b,
                )?;
            }
            InstructionKind::Subtract => {
                total_time += self.apply_operation(
                    argument_values,
                    registers,
                    [0, 1, 2],
                    |a, b| a.checked_sub(b),
                    |a, b| a - b,
                )?;
            }
            InstructionKind::Negate => {
                total_time += self.write_to_argument(registers, 0, -argument_values[0].unwrap())?;
            }
            InstructionKind::Multiply => {
                total_time += self.apply_operation(
                    argument_values,
                    registers,
                    [0, 1, 2],
                    |a, b| a.checked_mul(b),
                    |a, b| a * b,
                )?;
            }
            InstructionKind::Divide => {
                total_time += self.apply_operation(
                    argument_values,
                    registers,
                    [0, 1, 2],
                    |a, b| a.checked_div_euclid(b),
                    |a, b| {
                        a.checked_div_euclid(b)
                            .ok_or(ArithmaticError::DivideByZero.into())
                    },
                )?;
            }
            InstructionKind::Modulus => {
                total_time += self.apply_operation(
                    argument_values,
                    registers,
                    [0, 1, 2],
                    |a, b| a.checked_rem_euclid(b),
                    |a, b| {
                        a.checked_div_euclid(b)
                            .ok_or(ArithmaticError::DivideByZero.into())
                    },
                )?;
            }
            InstructionKind::Compare => {
                total_time += self.write_to_argument(registers, 1, argument_values[0].unwrap())?;
            }
            InstructionKind::CompareSetIfTrue => {
                let result = argument_values[0].unwrap();

                if result == 1 {
                    total_time += self.write_to_argument(registers, 1, result)?;
                }
            }
            InstructionKind::CompareSetIfFalse => {
                let result = argument_values[0].unwrap();

                if result == 0 {
                    total_time += self.write_to_argument(registers, 1, result)?;
                }
            }
            InstructionKind::Jump
            | InstructionKind::JumpCondLikely
            | InstructionKind::JumpCondUnlikely => {
                if argument_values[0].is_none_or(|x| x != 0) {
                    jump = Some(*self.arguments[1].as_instruction().unwrap());
                }
            }
            InstructionKind::Sleep => {
                total_time += argument_values[0].unwrap().max(0) as u32;
            }
            InstructionKind::End => return Err(InstructionEvaluationInterrupt::ProgramComplete),
            InstructionKind::TryRead => (),
            InstructionKind::TryWrite => {
                let register = self.register_of_argument(0);

                total_time += registers
                    .get(register)
                    .ok_or_else(|| InstructionEvaluationInterrupt::RegisterError {
                        register,
                        error: RegisterAccessError::NoSuchRegister { got: register },
                    })?
                    .write_time;
            }
            InstructionKind::Clock => {
                let register_index = self.register_of_argument(0);

                let register = registers.get(register_index).ok_or_else(|| {
                    InstructionEvaluationInterrupt::RegisterError {
                        register: register_index,
                        error: RegisterAccessError::NoSuchRegister {
                            got: register_index,
                        },
                    }
                })?;

                let value = register.value().map_err(|error| {
                    InstructionEvaluationInterrupt::RegisterError {
                        register: register_index,
                        error,
                    }
                })?;

                let bound = value.maximum() as u64 + 1;

                let digits = argument_values[1].unwrap_or(0).max(0) as u32;

                let clock = {
                    if let Some(divisor) = 10u64.checked_pow(digits) {
                        (runtime / divisor % bound) as Integer
                    } else {
                        0
                    }
                };

                total_time += self.write_to_argument(registers, 0, clock)?;
            }
        }

        if let Some(jump) = jump {
            *next_instruction = jump;
        } else {
            *next_instruction += 1;
        }

        Ok((total_time, argument_values, update_previous_instruction))
    }

    /// Returns the time to evaluate the instruction and whether or not to update the
    /// `previous_instruction`.
    #[must_use]
    pub fn execution_time(
        &self,
        previous_instruction: Option<(&Instruction, &ArgumentValues)>,
        argument_values: &ArgumentValues,
    ) -> (u32, bool) {
        let properties = self.kind.get_properties();

        if let &Some((time, ref condition)) = &properties.conditional_time {
            if condition.matches_context(previous_instruction, self.arguments, argument_values) {
                return (time, !condition.allows_cascade());
            }
        }

        (properties.base_time, true)
    }

    pub fn group(
        &self,
        previous_instruction: Option<(&Instruction, &ArgumentValues)>,
    ) -> InstructionKind {
        let properties = self.kind.get_properties();

        if let &Some((group, ref condition)) = &properties.group {
            if condition.matches_context(
                previous_instruction,
                self.arguments,
                &ArgumentValues::default(),
            ) {
                return group;
            }
        }

        self.kind
    }

    /// Returns the amount of time taken by the instruction, or an interrupt.
    ///
    /// # Panics
    ///
    /// Will panic if the argument does not contain a register
    fn write_to_argument(
        &self,
        registers: &mut RegisterSet,
        destination: usize,
        value: Integer,
    ) -> Result<u32, InstructionEvaluationInterrupt> {
        let register = self.register_of_argument(destination);

        registers
            .buffered_write(register, value)
            .map_err(|error| InstructionEvaluationInterrupt::RegisterError { register, error })?;

        Ok(registers.get(register).unwrap().write_time)
    }

    /// # Panics
    ///
    /// Will panic if the argument does not contain a register
    #[must_use]
    fn register_of_argument(&self, argument: usize) -> u32 {
        self.arguments[argument].as_register().unwrap()
    }

    /// # Panics
    ///
    /// Will panic if the destination argument does not contain a register, or if the lhs or rhs
    /// arguments are not available.
    fn apply_operation<T>(
        &self,
        argument_values: [Option<Integer>; Instruction::NUM_ARGUMENTS],
        registers: &mut RegisterSet,
        argument_sources: [usize; 3],
        fast_function: impl FnOnce(Integer, Integer) -> Option<Integer>,
        debug_function: impl FnOnce(BiggerInteger, BiggerInteger) -> T,
    ) -> Result<u32, InstructionEvaluationInterrupt>
    where
        T: IntoDebugResult,
    {
        let lhs = argument_values[argument_sources[0]].unwrap();
        let rhs = argument_values[argument_sources[1]].unwrap();

        if let Some(result) = fast_function(lhs, rhs) {
            Ok(self.write_to_argument(registers, argument_sources[2], result)?)
        } else {
            let result =
                debug_function(lhs as BiggerInteger, rhs as BiggerInteger).into_debug_result()?;

            let register = self.register_of_argument(argument_sources[2]);
            let value = registers.get(register).unwrap().value().map_err(|error| {
                InstructionEvaluationInterrupt::RegisterError { register, error }
            })?;

            Err(InstructionEvaluationInterrupt::RegisterError {
                register,
                error: RegisterAccessError::InvalidAssignment {
                    error: if result > 0 {
                        AssignIntegerError::ValueMuchTooBig {
                            got: result,
                            maximum: value.maximum(),
                        }
                    } else {
                        AssignIntegerError::ValueMuchTooSmall {
                            got: result,
                            minimum: value.minimum(),
                        }
                    },
                },
            })
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind.get_properties().name)?;

        for argument in self.arguments {
            if argument.is_empty() {
                continue;
            }

            write!(f, " {argument}")?;
        }

        Ok(())
    }
}

trait IntoDebugResult {
    fn into_debug_result(self) -> Result<BiggerInteger, InstructionEvaluationInterrupt>;
}

impl IntoDebugResult for Result<BiggerInteger, InstructionEvaluationInterrupt> {
    fn into_debug_result(self) -> Result<BiggerInteger, InstructionEvaluationInterrupt> {
        self
    }
}

impl IntoDebugResult for BiggerInteger {
    fn into_debug_result(self) -> Result<BiggerInteger, InstructionEvaluationInterrupt> {
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructionKind {
    Set,
    Add,
    Subtract,
    Negate,
    Multiply,
    Divide,
    Modulus,
    Compare,
    CompareSetIfTrue,
    CompareSetIfFalse,
    Jump,
    JumpCondLikely,
    JumpCondUnlikely,
    Sleep,
    End,
    TryRead,
    TryWrite,
    Clock,
}

impl InstructionKind {
    pub const fn get_properties(self) -> &'static InstructionKindProperties {
        INSTRUCTION_KINDS.get(self)
    }
}

impl FromStr for InstructionKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 3 {
            return Err(());
        }

        for properties in &INSTRUCTION_KINDS.0 {
            if properties.name == s {
                return Ok(properties.kind);
            }
        }

        Err(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InstructionKindMap<T>(pub [T; 18]);

impl<T> InstructionKindMap<T> {
    pub fn from_element(element: T) -> Self
    where
        T: Clone,
    {
        Self(array::from_fn(|_| element.clone()))
    }

    pub const fn get(&self, kind: InstructionKind) -> &T {
        &self.0[kind as usize]
    }

    pub const fn get_mut(&mut self, kind: InstructionKind) -> &mut T {
        &mut self.0[kind as usize]
    }
}

impl<T> Index<InstructionKind> for InstructionKindMap<T> {
    type Output = T;

    fn index(&self, index: InstructionKind) -> &Self::Output {
        self.get(index)
    }
}

impl<T> IndexMut<InstructionKind> for InstructionKindMap<T> {
    fn index_mut(&mut self, index: InstructionKind) -> &mut Self::Output {
        self.get_mut(index)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InstructionEvaluationInterrupt {
    RegisterError {
        register: u32,
        error: RegisterAccessError,
    },
    ArithmaticError {
        error: ArithmaticError,
    },
    ProgramComplete,
    RuntimeCounterOverflow,
}

impl From<ArithmaticError> for InstructionEvaluationInterrupt {
    fn from(error: ArithmaticError) -> Self {
        InstructionEvaluationInterrupt::ArithmaticError { error }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ArithmaticError {
    DivideByZero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentRequirement {
    Constant,
    RegisterWriteOnly,
    Register,
    ConstantOrRegister,
    Comparison,
    AnyValue,
    AnyValueOrEmpty,
    Instruction,
    ConstantOrEmpty,
    Empty,
}

impl ArgumentRequirement {
    pub fn allows_empty(self) -> bool {
        matches!(
            self,
            ArgumentRequirement::Empty
                | ArgumentRequirement::AnyValueOrEmpty
                | ArgumentRequirement::ConstantOrEmpty
        )
    }
}

impl Display for ArgumentRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ArgumentRequirement::Constant => "a constant",
                ArgumentRequirement::RegisterWriteOnly | ArgumentRequirement::Register =>
                    "a register",
                ArgumentRequirement::ConstantOrRegister => "a constant or register",
                ArgumentRequirement::Comparison => "a comparison",
                ArgumentRequirement::AnyValue => "a constant, register, or comparison",
                ArgumentRequirement::AnyValueOrEmpty =>
                    "a constant, register, comparison, or nothing",
                ArgumentRequirement::Instruction => "a label",
                ArgumentRequirement::ConstantOrEmpty => "a constant or nothing",
                ArgumentRequirement::Empty => "nothing",
            }
        )
    }
}

pub struct InstructionKindProperties {
    pub kind: InstructionKind,
    pub name: &'static str,
    pub arguments: [ArgumentRequirement; Instruction::NUM_ARGUMENTS],
    pub base_time: u32,
    pub conditional_time: Option<(u32, PropertyCondition)>,
    pub calls_per_tick_limit: Option<NonZeroU8>,
    pub group: Option<(InstructionKind, PropertyCondition)>,
}

impl InstructionKindProperties {
    pub const DEFAULT: Self = Self {
        kind: InstructionKind::Set,
        name: "",
        arguments: [ArgumentRequirement::Empty; Instruction::NUM_ARGUMENTS],
        base_time: 0,
        conditional_time: None,
        calls_per_tick_limit: Some(NonZeroU8::new(1).unwrap()),
        group: None,
    };

    pub fn minimum_arguments(&self) -> usize {
        self.arguments
            .iter()
            .filter(|requirement| {
                !matches!(
                    requirement,
                    ArgumentRequirement::Empty
                        | ArgumentRequirement::ConstantOrEmpty
                        | ArgumentRequirement::AnyValueOrEmpty
                )
            })
            .count()
    }

    pub fn maximum_arguments(&self) -> usize {
        self.arguments
            .iter()
            .filter(|requirement| !matches!(requirement, ArgumentRequirement::Empty))
            .count()
    }
}

impl Default for InstructionKindProperties {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub enum PropertyCondition {
    SameAsPrevious {
        kind: InstructionKind,
        allow_cascade: bool,
    },
    ArgumentMatches {
        argument: usize,
        value: Integer,
    },
    ArgumentTypeMatches {
        argument: usize,
        requirement: ArgumentRequirement,
    },
    Always,
}

impl PropertyCondition {
    #[must_use]
    pub fn matches_context<const N: usize>(
        &self,
        previous_instruction: Option<(&Instruction, &ArgumentValues)>,
        arguments: [Argument; N],
        argument_values: &ArgumentValues,
    ) -> bool {
        match self {
            PropertyCondition::SameAsPrevious { kind, .. } => previous_instruction.is_some_and(
                |(previous_instruction, previous_argument_values)| {
                    previous_instruction.kind == *kind
                        && previous_argument_values == argument_values
                },
            ),
            PropertyCondition::ArgumentMatches { argument, value } => {
                argument_values[*argument] == Some(*value)
            }
            PropertyCondition::ArgumentTypeMatches {
                argument,
                requirement,
            } => arguments[*argument].matches_requirement(*requirement),
            PropertyCondition::Always => true,
        }
    }

    pub fn allows_cascade(&self) -> bool {
        match self {
            PropertyCondition::SameAsPrevious { allow_cascade, .. } => *allow_cascade,
            _ => true,
        }
    }
}

const fn arguments<const N: usize>(
    arguments: [ArgumentRequirement; N],
) -> [ArgumentRequirement; Instruction::NUM_ARGUMENTS] {
    let mut all_arguments = [ArgumentRequirement::Empty; Instruction::NUM_ARGUMENTS];

    let mut i = 0;

    while i < arguments.len() {
        all_arguments[i] = arguments[i];

        i += 1;
    }

    all_arguments
}

const _: () = {
    let mut i = 0;

    while i < INSTRUCTION_KINDS.0.len() {
        let expected_kind = unsafe { std::mem::transmute::<u8, InstructionKind>(i as u8) };
        let stored_kind = expected_kind.get_properties().kind;

        assert!(
            expected_kind as u8 == stored_kind as u8,
            "Properties stored in INSTRUCTION_KINDS do not match their indecies",
        );

        i += 1;
    }
};

#[allow(clippy::needless_update)]
pub static INSTRUCTION_KINDS: InstructionKindMap<InstructionKindProperties> = InstructionKindMap([
    InstructionKindProperties {
        kind: InstructionKind::Set,
        name: "SET",
        arguments: arguments([
            ArgumentRequirement::RegisterWriteOnly,
            ArgumentRequirement::ConstantOrRegister,
        ]),
        base_time: 1,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Add,
        name: "ADD",
        arguments: arguments([
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 1,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Subtract,
        name: "SUB",
        arguments: arguments([
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 1,
        group: Some((
            InstructionKind::Negate,
            PropertyCondition::ArgumentTypeMatches {
                argument: 1,
                requirement: ArgumentRequirement::Register,
            },
        )),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Negate,
        name: "NEG",
        arguments: arguments([ArgumentRequirement::Register]),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Multiply,
        name: "MUL",
        arguments: arguments([
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 2,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Divide,
        name: "DIV",
        arguments: arguments([
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 4,
        conditional_time: Some((
            1,
            PropertyCondition::SameAsPrevious {
                kind: InstructionKind::Modulus,
                allow_cascade: false,
            },
        )),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Modulus,
        name: "MOD",
        arguments: arguments([
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 4,
        conditional_time: Some((
            1,
            PropertyCondition::SameAsPrevious {
                kind: InstructionKind::Divide,
                allow_cascade: false,
            },
        )),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Compare,
        name: "CMP",
        arguments: arguments([
            ArgumentRequirement::Comparison,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 1,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::CompareSetIfTrue,
        name: "TCP",
        arguments: arguments([
            ArgumentRequirement::Comparison,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 2,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::CompareSetIfFalse,
        name: "FCP",
        arguments: arguments([
            ArgumentRequirement::Comparison,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 2,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Jump,
        name: "JMP",
        arguments: arguments([
            ArgumentRequirement::AnyValueOrEmpty,
            ArgumentRequirement::Instruction,
        ]),
        base_time: 1,
        conditional_time: Some((
            0,
            PropertyCondition::ArgumentTypeMatches {
                argument: 0,
                requirement: ArgumentRequirement::ConstantOrEmpty,
            },
        )),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::JumpCondLikely,
        name: "LJP",
        arguments: arguments([
            ArgumentRequirement::AnyValue,
            ArgumentRequirement::Instruction,
        ]),
        base_time: 0,
        conditional_time: Some((
            5,
            PropertyCondition::ArgumentMatches {
                argument: 0,
                value: 0,
            },
        )),
        group: Some((InstructionKind::Jump, PropertyCondition::Always)),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::JumpCondUnlikely,
        name: "UJP",
        arguments: arguments([
            ArgumentRequirement::AnyValue,
            ArgumentRequirement::Instruction,
        ]),
        base_time: 5,
        conditional_time: Some((
            0,
            PropertyCondition::ArgumentMatches {
                argument: 0,
                value: 0,
            },
        )),
        group: Some((InstructionKind::Jump, PropertyCondition::Always)),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Sleep,
        name: "SLP",
        arguments: arguments([ArgumentRequirement::ConstantOrRegister]),
        calls_per_tick_limit: None,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::End,
        name: "END",
        arguments: arguments([]),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::TryRead,
        name: "TRY",
        arguments: arguments([ArgumentRequirement::Register]),
        calls_per_tick_limit: None,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::TryWrite,
        name: "TRW",
        arguments: arguments([ArgumentRequirement::RegisterWriteOnly]),
        calls_per_tick_limit: None,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Clock,
        name: "CLK",
        arguments: arguments([
            ArgumentRequirement::RegisterWriteOnly,
            ArgumentRequirement::ConstantOrEmpty,
        ]),
        ..InstructionKindProperties::DEFAULT
    },
]);
