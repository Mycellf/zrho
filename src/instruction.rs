use std::{
    array,
    fmt::Display,
    num::NonZeroU8,
    ops::{Index, IndexMut},
};

use crate::{
    argument::Argument,
    computer::{RegisterAccessError, RegisterSet},
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
        instruction: &mut u32,
    ) -> Result<(u32, ArgumentValues, bool), InstructionEvaluationInterrupt> {
        let properties = self.kind.get_properties();

        let mut total_time = 0;

        let mut argument_values = [None; 3];

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

                    total_time += register.map_or(0, |register| register.read_time);
                    Some(value.get())
                }
                Argument::Comparison(comparison) => {
                    let (value, registers) = comparison.evaluate(registers)?;

                    for register in registers.into_iter().flatten() {
                        total_time += register.read_time;
                    }
                    Some(value)
                }
                Argument::Empty => None,
            };
        }

        let (instruction_time, update_previous_instruction) =
            self.execution_time(previous_instruction, &argument_values);

        total_time += instruction_time;

        let mut jump = None;

        match self.kind {
            InstructionKind::Set => {
                total_time += self.write_to_argument(registers, 0, argument_values[1].unwrap())?;
            }
            InstructionKind::Try => (),
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
        }

        if let Some(jump) = jump {
            *instruction = jump;
        } else {
            *instruction += 1;
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
            match condition {
                TimeCondition::SameAsPrevious {
                    kind,
                    allow_cascade,
                } => 'outer: {
                    let Some((previous_instruction, previous_argument_values)) =
                        previous_instruction
                    else {
                        break 'outer;
                    };

                    if previous_instruction.kind == *kind
                        && previous_argument_values == argument_values
                    {
                        return (time, *allow_cascade);
                    }
                }
                TimeCondition::ArgumentMatches { argument, value } => {
                    if argument_values[*argument] == Some(*value) {
                        return (time, true);
                    }
                }
                TimeCondition::ArgumentTypeMatches {
                    argument,
                    requirement,
                } => {
                    if self.arguments[*argument].matches_requirement(*requirement) {
                        return (time, true);
                    }
                }
            }
        }

        (properties.base_time, true)
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

        Ok(registers
            .write(register, value)
            .map_err(|error| InstructionEvaluationInterrupt::RegisterError { register, error })?
            .write_time)
    }

    /// # Panics
    ///
    /// Will panic if the argument does not contain a register
    #[must_use]
    fn register_of_argument(&self, argument: usize) -> u32 {
        *self.arguments[argument]
            .as_number()
            .unwrap()
            .as_register()
            .unwrap()
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
    Try,
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
}

impl InstructionKind {
    pub fn get_properties(self) -> &'static InstructionKindProperties {
        &INSTRUCTION_KINDS[self]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InstructionKindMap<T>(pub [T; 16]);

impl<T> InstructionKindMap<T> {
    pub fn from_element(element: T) -> Self
    where
        T: Clone,
    {
        Self(array::from_fn(|_| element.clone()))
    }

    pub fn get(&self, kind: InstructionKind) -> &T {
        &self.0[kind as usize]
    }

    pub fn get_mut(&mut self, kind: InstructionKind) -> &mut T {
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

pub struct InstructionKindProperties {
    pub kind: InstructionKind,
    pub name: &'static str,
    pub arguments: [ArgumentRequirement; Instruction::NUM_ARGUMENTS],
    pub base_time: u32,
    pub conditional_time: Option<(u32, TimeCondition)>,
    pub calls_per_tick_limit: Option<NonZeroU8>,
}

impl InstructionKindProperties {
    pub const DEFAULT: Self = Self {
        kind: InstructionKind::Set,
        name: "",
        arguments: [ArgumentRequirement::Empty; Instruction::NUM_ARGUMENTS],
        base_time: 0,
        conditional_time: None,
        calls_per_tick_limit: Some(NonZeroU8::new(1).unwrap()),
    };
}

impl Default for InstructionKindProperties {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub enum TimeCondition {
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
}

#[allow(clippy::needless_update)]
pub static INSTRUCTION_KINDS: InstructionKindMap<InstructionKindProperties> = InstructionKindMap([
    InstructionKindProperties {
        kind: InstructionKind::Set,
        name: "SET",
        arguments: [
            ArgumentRequirement::RegisterWriteOnly,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::Empty,
        ],
        base_time: 1,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Try,
        name: "TRY",
        arguments: [
            ArgumentRequirement::Register,
            ArgumentRequirement::Empty,
            ArgumentRequirement::Empty,
        ],
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Add,
        name: "ADD",
        arguments: [
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ],
        base_time: 1,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Subtract,
        name: "SUB",
        arguments: [
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ],
        base_time: 1,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Negate,
        name: "NEG",
        arguments: [
            ArgumentRequirement::Register,
            ArgumentRequirement::Empty,
            ArgumentRequirement::Empty,
        ],
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Multiply,
        name: "MUL",
        arguments: [
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ],
        base_time: 2,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Divide,
        name: "DIV",
        arguments: [
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ],
        base_time: 4,
        conditional_time: Some((
            1,
            TimeCondition::SameAsPrevious {
                kind: InstructionKind::Modulus,
                allow_cascade: false,
            },
        )),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Modulus,
        name: "MOD",
        arguments: [
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWriteOnly,
        ],
        base_time: 4,
        conditional_time: Some((
            1,
            TimeCondition::SameAsPrevious {
                kind: InstructionKind::Divide,
                allow_cascade: false,
            },
        )),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Compare,
        name: "CMP",
        arguments: [
            ArgumentRequirement::Comparison,
            ArgumentRequirement::RegisterWriteOnly,
            ArgumentRequirement::Empty,
        ],
        base_time: 1,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::CompareSetIfTrue,
        name: "TCP",
        arguments: [
            ArgumentRequirement::Comparison,
            ArgumentRequirement::RegisterWriteOnly,
            ArgumentRequirement::Empty,
        ],
        base_time: 2,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::CompareSetIfFalse,
        name: "FCP",
        arguments: [
            ArgumentRequirement::Comparison,
            ArgumentRequirement::RegisterWriteOnly,
            ArgumentRequirement::Empty,
        ],
        base_time: 2,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Jump,
        name: "JMP",
        arguments: [
            ArgumentRequirement::AnyValueOrEmpty,
            ArgumentRequirement::Instruction,
            ArgumentRequirement::Empty,
        ],
        base_time: 1,
        conditional_time: Some((
            0,
            TimeCondition::ArgumentTypeMatches {
                argument: 0,
                requirement: ArgumentRequirement::ConstantOrEmpty,
            },
        )),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::JumpCondLikely,
        name: "LJP",
        arguments: [
            ArgumentRequirement::AnyValue,
            ArgumentRequirement::Instruction,
            ArgumentRequirement::Empty,
        ],
        base_time: 0,
        conditional_time: Some((
            5,
            TimeCondition::ArgumentMatches {
                argument: 0,
                value: 0,
            },
        )),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::JumpCondUnlikely,
        name: "UJP",
        arguments: [
            ArgumentRequirement::AnyValue,
            ArgumentRequirement::Instruction,
            ArgumentRequirement::Empty,
        ],
        base_time: 5,
        conditional_time: Some((
            0,
            TimeCondition::ArgumentMatches {
                argument: 0,
                value: 0,
            },
        )),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Sleep,
        name: "SLP",
        arguments: [
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::Empty,
            ArgumentRequirement::Empty,
        ],
        calls_per_tick_limit: None,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::End,
        name: "END",
        arguments: [
            ArgumentRequirement::Empty,
            ArgumentRequirement::Empty,
            ArgumentRequirement::Empty,
        ],
        ..InstructionKindProperties::DEFAULT
    },
]);
