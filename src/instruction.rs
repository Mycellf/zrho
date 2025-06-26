use std::cmp::Ordering;

use crate::integer::{DigitInteger, Integer};

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
            ArgumentRequirement::Register => {
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

#[derive(Clone, Copy, Debug)]
pub struct Comparison {
    pub ordering: Ordering,
    pub invert: bool,
    pub values: [NumberSource; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentRequirement {
    Constant,
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
}

impl InstructionKind {
    pub fn get_properties(self) -> &'static InstructionKindProperties {
        &INSTRUCTION_KINDS[self as usize]
    }
}

pub static INSTRUCTION_KINDS: [InstructionKindProperties; 15] = [
    InstructionKindProperties {
        kind: InstructionKind::Set,
        name: "SET",
        arguments: [
            ArgumentRequirement::Register,
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
            ArgumentRequirement::Register,
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
            ArgumentRequirement::Register,
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
            ArgumentRequirement::Register,
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
            ArgumentRequirement::Register,
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
            ArgumentRequirement::Register,
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
            ArgumentRequirement::Register,
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
            ArgumentRequirement::Register,
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
            ArgumentRequirement::Register,
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
];
