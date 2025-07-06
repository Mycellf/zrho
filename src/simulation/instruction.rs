use std::{
    array,
    fmt::Display,
    num::NonZeroU8,
    ops::{Index, IndexMut},
};

use strum::{EnumCount, EnumIter, VariantArray};

use crate::simulation::computer::Register;

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

    /// Returns the total time and energy taken by the instruction, the value of each input, and
    /// whether or not to update `previous_instruction`.
    pub fn evaluate(
        &self,
        registers: &mut RegisterSet,
        instruction_properties: &InstructionKindMap<InstructionProperties>,
        previous_instruction: Option<(&Instruction, &ArgumentValues)>,
        next_instruction: &mut u32,
        runtime: u64,
    ) -> Result<(u32, u32, ArgumentValues, bool), InstructionEvaluationInterrupt> {
        let properties = instruction_properties[self.kind];

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
                read_time = read_time.max({
                    let register = registers.get(register as u32).unwrap();

                    register.read_time + register.block_time
                });
            }
        }

        let (mut instruction_time, instruction_energy, update_previous_instruction) = self
            .execution_time(
                instruction_properties,
                previous_instruction,
                &argument_values,
            );

        let mut jump = None;

        let mut write_time = 0;
        let mut write_block_time = 0;

        match self.kind {
            InstructionKind::Set => {
                self.write_to_argument(registers, 0, argument_values[1].unwrap())?
                    .set_time_to_write(&mut write_time, &mut write_block_time);
            }
            InstructionKind::Add => {
                self.apply_operation(
                    argument_values,
                    registers,
                    [0, 1, 2],
                    |a, b| a.checked_add(b),
                    |a, b| a + b,
                )?
                .set_time_to_write(&mut write_time, &mut write_block_time);
            }
            InstructionKind::Subtract => {
                self.apply_operation(
                    argument_values,
                    registers,
                    [0, 1, 2],
                    |a, b| a.checked_sub(b),
                    |a, b| a - b,
                )?
                .set_time_to_write(&mut write_time, &mut write_block_time);
            }
            InstructionKind::Negate => {
                self.write_to_argument(registers, 0, -argument_values[0].unwrap())?
                    .set_time_to_write(&mut write_time, &mut write_block_time);
            }
            InstructionKind::Multiply => {
                self.apply_operation(
                    argument_values,
                    registers,
                    [0, 1, 2],
                    |a, b| a.checked_mul(b),
                    |a, b| a * b,
                )?
                .set_time_to_write(&mut write_time, &mut write_block_time);
            }
            InstructionKind::Divide => {
                self.apply_operation(
                    argument_values,
                    registers,
                    [0, 1, 2],
                    |a, b| a.checked_div_euclid(b),
                    |a, b| {
                        a.checked_div_euclid(b)
                            .ok_or(ArithmaticError::DivideByZero.into())
                    },
                )?
                .set_time_to_write(&mut write_time, &mut write_block_time);
            }
            InstructionKind::Modulus => {
                self.apply_operation(
                    argument_values,
                    registers,
                    [0, 1, 2],
                    |a, b| a.checked_rem_euclid(b),
                    |a, b| {
                        a.checked_div_euclid(b)
                            .ok_or(ArithmaticError::DivideByZero.into())
                    },
                )?
                .set_time_to_write(&mut write_time, &mut write_block_time);
            }
            InstructionKind::IsOdd => {
                self.write_to_argument(registers, 0, argument_values[0].unwrap().rem_euclid(2))?
                    .set_time_to_write(&mut write_time, &mut write_block_time);
            }
            InstructionKind::Compare => {
                self.write_to_argument(registers, 1, argument_values[0].unwrap())?
                    .set_time_to_write(&mut write_time, &mut write_block_time);
            }
            InstructionKind::CompareSetIfTrue => {
                let result = argument_values[0].unwrap();

                if result == 1 {
                    self.write_to_argument(registers, 1, result)?
                        .set_time_to_write(&mut write_time, &mut write_block_time);
                }
            }
            InstructionKind::CompareSetIfFalse => {
                let result = argument_values[0].unwrap();

                if result == 0 {
                    self.write_to_argument(registers, 1, result)?
                        .set_time_to_write(&mut write_time, &mut write_block_time);
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
                instruction_time += argument_values[0].unwrap().max(0) as u32;
            }
            InstructionKind::End => return Err(InstructionEvaluationInterrupt::ProgramComplete),
            InstructionKind::TryRead => (),
            InstructionKind::TryWrite => {
                let register = self.register_of_argument(0);

                registers
                    .get(register)
                    .ok_or(InstructionEvaluationInterrupt::RegisterError {
                        register,
                        error: RegisterAccessError::NoSuchRegister { got: register },
                    })?
                    .set_time_to_write(&mut write_time, &mut write_block_time);
            }
            InstructionKind::Clock => {
                let register_index = self.register_of_argument(0);

                let register = registers.get(register_index).ok_or({
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

                self.write_to_argument(registers, 0, clock)?
                    .set_time_to_write(&mut write_time, &mut write_block_time);
            }
        }

        let total_time = (read_time + instruction_time).max(write_block_time) + write_time;

        if let Some(jump) = jump {
            *next_instruction = jump;
        } else {
            *next_instruction += 1;
        }

        Ok((
            total_time,
            instruction_energy,
            argument_values,
            update_previous_instruction,
        ))
    }

    /// Returns the time to evaluate the instruction and whether or not to update the
    /// `previous_instruction`.
    #[must_use]
    pub fn execution_time(
        &self,
        instruction_properties: &InstructionKindMap<InstructionProperties>,
        previous_instruction: Option<(&Instruction, &ArgumentValues)>,
        argument_values: &ArgumentValues,
    ) -> (u32, u32, bool) {
        let properties = instruction_properties[self.kind];

        if let &Some((time, ref condition)) = &properties.conditional_time {
            if condition.matches_context(previous_instruction, self.arguments, argument_values) {
                let energy = properties
                    .conditional_energy
                    .unwrap_or(properties.base_energy);

                return (time, energy, !condition.allows_cascade());
            }
        }

        (properties.base_time, properties.base_energy, true)
    }

    pub fn group(
        &self,
        instruction_properties: &InstructionKindMap<InstructionProperties>,
        previous_instruction: Option<(&Instruction, &ArgumentValues)>,
    ) -> InstructionKind {
        let properties = instruction_properties[self.kind];

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
    fn write_to_argument<'a>(
        &self,
        registers: &'a mut RegisterSet,
        destination: usize,
        value: Integer,
    ) -> Result<&'a Register, InstructionEvaluationInterrupt> {
        let register = self.register_of_argument(destination);

        registers
            .buffered_write(register, value)
            .map_err(|error| InstructionEvaluationInterrupt::RegisterError { register, error })?;

        Ok(registers.get(register).unwrap())
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
    fn apply_operation<'a, T>(
        &self,
        argument_values: [Option<Integer>; Instruction::NUM_ARGUMENTS],
        registers: &'a mut RegisterSet,
        argument_sources: [usize; 3],
        fast_function: impl FnOnce(Integer, Integer) -> Option<Integer>,
        debug_function: impl FnOnce(BiggerInteger, BiggerInteger) -> T,
    ) -> Result<&'a Register, InstructionEvaluationInterrupt>
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
        write!(f, "{}", self.kind.get_default_properties().name)?;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, EnumCount, VariantArray)]
pub enum InstructionKind {
    Set,
    Add,
    Subtract,
    Negate,
    Multiply,
    Divide,
    Modulus,
    IsOdd,
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
    pub const fn get_default_properties(self) -> &'static InstructionProperties {
        DEFAULT_INSTRUCTIONS.get(self)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InstructionKindMap<T>(pub [T; InstructionKind::COUNT]);

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

impl InstructionKindMap<InstructionProperties> {
    /// # Panics
    ///
    /// Will panic if the function modifies the `kind` field of the passed properties.
    pub fn with_instruction<F>(mut self, kind: InstructionKind, function: F) -> Self
    where
        F: FnOnce(&mut InstructionProperties),
    {
        let properties = &mut self[kind];
        function(properties);

        assert_eq!(properties.kind, kind);

        self
    }

    pub fn instruction_with_name(&self, name: &str) -> Option<&InstructionProperties> {
        if name.is_empty() {
            return None;
        }

        self.0.iter().find(|&properties| properties.name == name)
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
    EnergyCounterOverflow,
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

#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct InstructionProperties {
    pub kind: InstructionKind,
    pub name: &'static str,
    pub arguments: [ArgumentRequirement; Instruction::NUM_ARGUMENTS],
    pub base_time: u32,
    pub conditional_time: Option<(u32, PropertyCondition)>,
    pub base_energy: u32,
    pub conditional_energy: Option<u32>,
    pub calls_per_tick_limit: Option<NonZeroU8>,
    pub group: Option<(InstructionKind, PropertyCondition)>,
}

impl InstructionProperties {
    pub const DEFAULT: Self = Self {
        kind: InstructionKind::Set,
        name: "",
        arguments: [ArgumentRequirement::Empty; Instruction::NUM_ARGUMENTS],
        base_time: 0,
        conditional_time: None,
        base_energy: 0,
        conditional_energy: None,
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

impl Default for InstructionProperties {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone, Copy, Debug)]
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

    while i < DEFAULT_INSTRUCTIONS.0.len() {
        let expected_kind = InstructionKind::VARIANTS[i];
        let stored_kind = expected_kind.get_default_properties().kind;

        assert!(
            expected_kind as u8 == stored_kind as u8,
            "Properties stored in INSTRUCTION_KINDS do not match their indecies",
        );

        i += 1;
    }
};

pub static DEFAULT_INSTRUCTIONS: InstructionKindMap<InstructionProperties> = InstructionKindMap([
    InstructionProperties {
        kind: InstructionKind::Set,
        name: "SET",
        arguments: arguments([
            ArgumentRequirement::RegisterWriteOnly,
            ArgumentRequirement::ConstantOrRegister,
        ]),
        base_time: 1,
        base_energy: 1,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::Add,
        name: "ADD",
        arguments: arguments([
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 1,
        base_energy: 2,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::Subtract,
        name: "SUB",
        arguments: arguments([
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 1,
        base_energy: 2,
        group: Some((
            InstructionKind::Negate,
            PropertyCondition::ArgumentTypeMatches {
                argument: 1,
                requirement: ArgumentRequirement::Register,
            },
        )),
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::Negate,
        name: "NEG",
        arguments: arguments([ArgumentRequirement::Register]),
        base_energy: 1,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::Multiply,
        name: "MUL",
        arguments: arguments([
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 2,
        base_energy: 4,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
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
        base_energy: 8,
        conditional_energy: Some(0),
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
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
        base_energy: 8,
        conditional_energy: Some(0),
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::IsOdd,
        name: "ODD",
        arguments: arguments([ArgumentRequirement::Register]),
        base_time: 0,
        base_energy: 1,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::Compare,
        name: "CMP",
        arguments: arguments([
            ArgumentRequirement::Comparison,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 1,
        base_energy: 1,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::CompareSetIfTrue,
        name: "TCP",
        arguments: arguments([
            ArgumentRequirement::Comparison,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 1,
        base_energy: 2,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::CompareSetIfFalse,
        name: "FCP",
        arguments: arguments([
            ArgumentRequirement::Comparison,
            ArgumentRequirement::RegisterWriteOnly,
        ]),
        base_time: 1,
        base_energy: 2,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
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
        base_energy: 1,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
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
        base_energy: 5,
        group: Some((InstructionKind::Jump, PropertyCondition::Always)),
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
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
        base_energy: 5,
        group: Some((InstructionKind::Jump, PropertyCondition::Always)),
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::Sleep,
        name: "SLP",
        arguments: arguments([ArgumentRequirement::ConstantOrRegister]),
        calls_per_tick_limit: None,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::End,
        name: "END",
        arguments: arguments([]),
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::TryRead,
        name: "TRY",
        arguments: arguments([ArgumentRequirement::Register]),
        calls_per_tick_limit: None,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::TryWrite,
        name: "TRW",
        arguments: arguments([ArgumentRequirement::RegisterWriteOnly]),
        calls_per_tick_limit: None,
        ..InstructionProperties::DEFAULT
    },
    InstructionProperties {
        kind: InstructionKind::Clock,
        name: "CLK",
        arguments: arguments([
            ArgumentRequirement::RegisterWriteOnly,
            ArgumentRequirement::ConstantOrEmpty,
        ]),
        base_energy: 2,
        ..InstructionProperties::DEFAULT
    },
]);
