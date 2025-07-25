use std::{cmp::Ordering, fmt::Display, slice};

use super::{
    computer::{self, RegisterAccessError, RegisterSet},
    instruction::{ArgumentRequirement, InstructionEvaluationInterrupt},
    integer::Integer,
};

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
    pub fn as_constant(&self) -> Option<Integer> {
        self.as_number()?.as_constant()
    }

    #[must_use]
    pub fn as_register(&self) -> Option<u32> {
        self.as_number()?.as_register()
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
            ArgumentRequirement::Register | ArgumentRequirement::RegisterWriteOnly => {
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
            ArgumentRequirement::ConstantOrEmpty => {
                matches!(
                    self,
                    Argument::Number(NumberSource::Constant(_)) | Argument::Empty
                )
            }
            ArgumentRequirement::Empty => matches!(self, Argument::Empty),
        }
    }

    #[must_use]
    pub fn number_sources(&self) -> &[NumberSource] {
        match self {
            Argument::Number(number_source) => slice::from_ref(number_source),
            Argument::Comparison(comparison) => &comparison.values,
            Argument::Instruction(_) | Argument::Empty => &[],
        }
    }
}

impl From<Integer> for Argument {
    fn from(constant: Integer) -> Self {
        Argument::Number(NumberSource::Constant(constant))
    }
}

impl From<u32> for Argument {
    fn from(register: u32) -> Self {
        Argument::Number(NumberSource::Register(register))
    }
}

impl From<NumberSource> for Argument {
    fn from(source: NumberSource) -> Self {
        Argument::Number(source)
    }
}

impl From<Comparison> for Argument {
    fn from(comparison: Comparison) -> Self {
        Self::Comparison(comparison)
    }
}

impl Display for Argument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Argument::Instruction(line) => write!(f, "INDEX_{line}"),
            Argument::Number(source) => write!(f, "{source}"),
            Argument::Comparison(comparison) => write!(f, "{comparison}"),
            Argument::Empty => write!(f, "_"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum NumberSource {
    Register(u32),
    Constant(Integer),
}

impl NumberSource {
    pub fn value(
        &self,
        registers: &RegisterSet,
    ) -> Result<(Integer, Option<u32>), InstructionEvaluationInterrupt> {
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
                    .map(|value| (value.get(), Some(*index)))
                    .map_err(|error| InstructionEvaluationInterrupt::RegisterError {
                        register: *index,
                        error,
                    })
            }
            NumberSource::Constant(value) => Ok((*value, None)),
        }
    }

    #[must_use]
    pub fn as_register(&self) -> Option<u32> {
        if let &Self::Register(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_constant(&self) -> Option<Integer> {
        if let &Self::Constant(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl Display for NumberSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberSource::Register(register) => {
                write!(f, "{}", computer::name_of_register(*register).unwrap())
            }
            NumberSource::Constant(value) => write!(f, "{value}"),
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
    pub fn evaluate(
        &self,
        registers: &RegisterSet,
    ) -> Result<(Integer, [Option<u32>; 2]), InstructionEvaluationInterrupt> {
        let (lhs, lhs_register) = self.values[0].value(registers)?;
        let (rhs, rhs_register) = self.values[1].value(registers)?;

        let result = (lhs.cmp(&rhs) == self.ordering) ^ self.invert;

        Ok((result as Integer, [lhs_register, rhs_register]))
    }
}

impl Display for Comparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SYMBOLS: [char; 6] = ['<', '=', '>', '≥', '≠', '≤'];
        let index = (self.ordering as isize + 1) as usize + self.invert as usize * 3;

        write!(
            f,
            "{lhs} {comparison} {rhs}",
            lhs = self.values[0],
            rhs = self.values[1],
            comparison = SYMBOLS[index],
        )
    }
}
