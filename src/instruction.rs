use std::cmp::Ordering;

use crate::{
    computer::{Register, RegisterAccessError, RegisterSet},
    integer::{DigitInteger, Integer},
};

pub type ArgumentValues = [Option<Integer>; Instruction::NUM_ARGUMENTS];

#[derive(Clone, Copy, Debug)]
pub struct Instruction {
    pub kind: InstructionKind,
    pub line: u32,
    arguments: [Argument; Self::NUM_ARGUMENTS],
}

impl Instruction {
    pub const NUM_ARGUMENTS: usize = 3;

    #[must_use]
    pub fn last_argument(&self) -> Option<usize> {
        self.arguments
            .into_iter()
            .enumerate()
            .rev()
            .find_map(|(i, argument)| argument.is_specified().then_some(i))
    }

    /// Returns the total time taken by the instruction and the values of each input
    pub fn evaluate(
        &self,
        registers: &mut RegisterSet,
        previous_instruction: Option<(&Instruction, &ArgumentValues)>,
    ) -> Result<(u32, ArgumentValues), InstructionEvaluationInterrupt> {
        let properties = self.kind.get_properties();

        let mut total_time = 0;

        let mut argument_values = [None; 3];

        for i in 0..argument_values.len() {
            let requirement = properties.arguments[i];

            assert!(self.arguments[i].matches_requirement(requirement));

            if matches!(requirement, ArgumentRequirement::RegisterWrite) {
                continue;
            }

            argument_values[i] = match self.arguments[i] {
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

        total_time += self.execution_time(previous_instruction, &argument_values);

        match self.kind {
            InstructionKind::Set => todo!(),
            InstructionKind::Try => (),
            InstructionKind::Add => todo!(),
            InstructionKind::Subtract => todo!(),
            InstructionKind::Negate => todo!(),
            InstructionKind::Multiply => todo!(),
            InstructionKind::Divide => todo!(),
            InstructionKind::Modulus => todo!(),
            InstructionKind::Compare => todo!(),
            InstructionKind::CompareSetIfTrue => todo!(),
            InstructionKind::CompareSetIfFalse => todo!(),
            InstructionKind::Jump => todo!(),
            InstructionKind::JumpCondLikely => todo!(),
            InstructionKind::JumpCondUnlikely => todo!(),
            InstructionKind::Sleep => {
                total_time += argument_values[0].unwrap().max(0) as u32;
            }
            InstructionKind::End => return Err(InstructionEvaluationInterrupt::ProgramComplete),
        }

        Ok((total_time, argument_values))
    }

    #[must_use]
    pub fn execution_time(
        &self,
        previous_instruction: Option<(&Instruction, &ArgumentValues)>,
        argument_values: &ArgumentValues,
    ) -> u32 {
        let properties = self.kind.get_properties();

        if let &Some((time, ref condition)) = &properties.conditional_time {
            match condition {
                TimeCondition::SameAsPrevious(instruction_kind) => 'outer: {
                    let Some((previous_instruction, previous_argument_values)) =
                        previous_instruction
                    else {
                        break 'outer;
                    };

                    if previous_instruction.kind == *instruction_kind
                        && previous_argument_values == argument_values
                    {
                        return time;
                    }
                }
                TimeCondition::ArgumentMatches { argument, value } => {
                    if argument_values[*argument] == Some(*value) {
                        return time;
                    }
                }
            }
        }

        properties.base_time
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InstructionEvaluationInterrupt {
    RegisterError {
        register: u32,
        error: RegisterAccessError,
    },
    ProgramComplete,
}

#[derive(Clone, Copy, Debug)]
pub enum Argument {
    Instruction(u32),
    Number(NumberSource),
    Comparison(Comparison),
    Empty,
}

impl Argument {
    /// Returns `true` unless the argument is [`Empty`].
    ///
    /// [`Empty`]: Argument::Empty
    #[must_use]
    pub fn is_specified(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns `true` if the argument is [`Empty`].
    ///
    /// [`Empty`]: Argument::Empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    #[must_use]
    pub fn as_instruction(&self) -> Option<&u32> {
        if let Self::Instruction(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_number(&self) -> Option<&NumberSource> {
        if let Self::Number(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_comparison(&self) -> Option<&Comparison> {
        if let Self::Comparison(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn matches_requirement(&self, requirement: ArgumentRequirement) -> bool {
        match requirement {
            ArgumentRequirement::Constant => {
                matches!(self, Argument::Number(NumberSource::Constant(_)))
            }
            ArgumentRequirement::Register | ArgumentRequirement::RegisterWrite => {
                matches!(self, Argument::Number(NumberSource::Register(_)))
            }
            ArgumentRequirement::ConstantOrRegister => matches!(self, Argument::Number(_)),
            ArgumentRequirement::Comparison => matches!(self, Argument::Comparison(_)),
            ArgumentRequirement::AnyValue => {
                matches!(self, Argument::Number(_) | Argument::Comparison(_))
            }
            ArgumentRequirement::AnyValueOrEmpty => {
                matches!(
                    self,
                    Argument::Number(_) | Argument::Comparison(_) | Argument::Empty
                )
            }
            ArgumentRequirement::Instruction => matches!(self, Argument::Instruction(_)),
            ArgumentRequirement::Empty => matches!(self, Argument::Empty),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum NumberSource {
    Register(u32),
    Constant(DigitInteger),
}

impl NumberSource {
    #[must_use]
    pub fn value<'a>(
        &self,
        registers: &'a RegisterSet,
    ) -> Result<(DigitInteger, Option<&'a Register>), InstructionEvaluationInterrupt> {
        match self {
            NumberSource::Register(index) => {
                let register =
                    registers
                        .get(*index)
                        .ok_or(InstructionEvaluationInterrupt::RegisterError {
                            register: *index,
                            error: RegisterAccessError::NoSuchRegister { got: *index },
                        })?;

                register
                    .value()
                    .map_err(|error| InstructionEvaluationInterrupt::RegisterError {
                        register: *index,
                        error,
                    })
                    .map(|value| (*value, Some(register)))
            }
            NumberSource::Constant(value) => Ok((*value, None)),
        }
    }

    #[must_use]
    pub fn as_register(&self) -> Option<&u32> {
        if let Self::Register(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_constant(&self) -> Option<&DigitInteger> {
        if let Self::Constant(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Comparison {
    pub ordering: Ordering,
    pub invert: bool,
    pub values: [NumberSource; 2],
}

impl Comparison {
    pub fn evaluate<'a>(
        &self,
        registers: &'a RegisterSet,
    ) -> Result<(Integer, [Option<&'a Register>; 2]), InstructionEvaluationInterrupt> {
        let (lhs, lhs_register) = self.values[0].value(registers)?;
        let (rhs, rhs_register) = self.values[1].value(registers)?;

        let result = (lhs.cmp(&rhs) == self.ordering) ^ self.invert;

        Ok((result as Integer, [lhs_register, rhs_register]))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentRequirement {
    Constant,
    RegisterWrite,
    Register,
    ConstantOrRegister,
    Comparison,
    AnyValue,
    AnyValueOrEmpty,
    Instruction,
    Empty,
}

pub struct InstructionKindProperties {
    pub kind: InstructionKind,
    pub name: &'static str,
    pub arguments: [ArgumentRequirement; Instruction::NUM_ARGUMENTS],
    pub base_time: u32,
    pub conditional_time: Option<(u32, TimeCondition)>,
}

impl InstructionKindProperties {
    pub const DEFAULT: Self = Self {
        kind: InstructionKind::Set,
        name: "",
        arguments: [ArgumentRequirement::Empty; Instruction::NUM_ARGUMENTS],
        base_time: 0,
        conditional_time: None,
    };
}

impl Default for InstructionKindProperties {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub enum TimeCondition {
    SameAsPrevious(InstructionKind),
    ArgumentMatches { argument: usize, value: Integer },
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
        &INSTRUCTION_KINDS[self as usize]
    }
}

pub static INSTRUCTION_KINDS: [InstructionKindProperties; 16] = [
    InstructionKindProperties {
        kind: InstructionKind::Set,
        name: "SET",
        arguments: [
            ArgumentRequirement::RegisterWrite,
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
            ArgumentRequirement::RegisterWrite,
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
            ArgumentRequirement::RegisterWrite,
        ],
        base_time: 1,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Subtract,
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
            ArgumentRequirement::RegisterWrite,
        ],
        base_time: 4,
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Divide,
        name: "DIV",
        arguments: [
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWrite,
        ],
        base_time: 8,
        conditional_time: Some((1, TimeCondition::SameAsPrevious(InstructionKind::Modulus))),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Modulus,
        name: "MOD",
        arguments: [
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::ConstantOrRegister,
            ArgumentRequirement::RegisterWrite,
        ],
        base_time: 8,
        conditional_time: Some((1, TimeCondition::SameAsPrevious(InstructionKind::Divide))),
        ..InstructionKindProperties::DEFAULT
    },
    InstructionKindProperties {
        kind: InstructionKind::Compare,
        name: "CMP",
        arguments: [
            ArgumentRequirement::Comparison,
            ArgumentRequirement::RegisterWrite,
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
            ArgumentRequirement::RegisterWrite,
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
            ArgumentRequirement::RegisterWrite,
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
];
