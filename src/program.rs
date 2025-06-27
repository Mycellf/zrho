use std::{array, cmp::Ordering, collections::HashMap, iter::Peekable, num::ParseIntError};

use crate::{
    argument::{Argument, Comparison, NumberSource},
    computer::{self, RegisterMap},
    instruction::{ArgumentRequirement, Instruction, InstructionKind},
};

pub const COMMENT_SEPARATOR: char = ';';
pub const LABEL_PSEUDO_INSTRUCTION: &str = "LBL";

#[derive(Clone, Debug)]
pub struct Program {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

impl Program {
    pub fn new_empty(name: String) -> Self {
        Self {
            name,
            instructions: Vec::new(),
        }
    }

    pub fn instruction(mut self, instruction: Instruction) -> Self {
        self.instructions.push(instruction);
        self
    }

    pub fn assemble_from(
        name: String,
        source_code: &str,
        allowed_registers: RegisterMap<bool>,
    ) -> Result<Self, Vec<ProgramAssemblyError>> {
        let mut errors = Vec::new();

        let mut program = Self::new_empty(name);

        let mut instructions = Vec::new();
        let mut labels = HashMap::new();

        for (i, line) in source_code.lines().enumerate() {
            match InstructionIntermediate::from_line(
                line,
                i.try_into().unwrap(),
                &allowed_registers,
            ) {
                Ok(instruction_result) => match instruction_result {
                    ParseInstructionResult::Instruction(instruction) => {
                        instructions.push(instruction);
                    }
                    ParseInstructionResult::Label(label) => {
                        labels.insert(label, u32::try_from(instructions.len()).unwrap());
                    }
                    ParseInstructionResult::Empty => (),
                },
                Err(error) => errors.push(error),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        for instruction in &mut instructions {
            for argument in &mut instruction.arguments {
                match argument {
                    ArgumentIntermediate::Label(label) => {
                        let Some(&index) = labels.get(label) else {
                            errors.push(ProgramAssemblyError {
                                line: instruction.line,
                                kind: ProgramAssemblyErrorKind::NoSuchLabel { got: label },
                            });

                            continue;
                        };

                        *argument = ArgumentIntermediate::Finished(Argument::Instruction(index));
                    }
                    ArgumentIntermediate::Finished(_) => (),
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        for instruction in instructions {
            program.instructions.push(instruction.complete().unwrap());
        }

        Ok(program)
    }
}

#[derive(Clone, Debug)]
pub struct ProgramAssemblyError<'a> {
    pub line: u32,
    pub kind: ProgramAssemblyErrorKind<'a>,
}

#[derive(Clone, Debug)]
pub enum ProgramAssemblyErrorKind<'a> {
    NoSuchRegister {
        got: u32,
    },
    NoSuchLabel {
        got: &'a str,
    },
    NoSuchOperation {
        got: &'a str,
    },
    InvalidArgument(ParseArgumentError<'a>),
    UnexpectedArgument {
        got: ArgumentIntermediate<'a>,
        expected: ArgumentRequirement,
    },
    TooManyArguments {
        got: ArgumentIntermediate<'a>,
    },
}

#[derive(Clone, Copy, Debug)]
struct InstructionIntermediate<'a> {
    kind: InstructionKind,
    line: u32,
    arguments: [ArgumentIntermediate<'a>; Instruction::NUM_ARGUMENTS],
}

enum ParseInstructionResult<'a> {
    Instruction(InstructionIntermediate<'a>),
    Label(&'a str),
    Empty,
}

#[derive(Clone, Copy, Debug)]
pub enum ArgumentIntermediate<'a> {
    Finished(Argument),
    Label(&'a str),
}

impl<'a> InstructionIntermediate<'a> {
    fn from_line(
        source_line: &'a str,
        index: u32,
        allowed_registers: &RegisterMap<bool>,
    ) -> Result<ParseInstructionResult<'a>, ProgramAssemblyError<'a>> {
        let line = source_line
            .split_once(COMMENT_SEPARATOR)
            .map(|(line, _)| line)
            .unwrap_or(source_line);

        let mut tokens = line.split_whitespace().peekable();

        let Some(instruction_code) = tokens.next() else {
            return Ok(ParseInstructionResult::Empty);
        };

        let mut next_argument = || match ArgumentIntermediate::pop_from_tokens(&mut tokens) {
            Ok(argument) => Ok(argument),
            Err(error) => Err(ProgramAssemblyError {
                line: index,
                kind: ProgramAssemblyErrorKind::InvalidArgument(error),
            }),
        };

        if instruction_code == LABEL_PSEUDO_INSTRUCTION {
            let argument = next_argument()?;

            let ArgumentIntermediate::Label(label) = argument else {
                return Err(ProgramAssemblyError {
                    line: index,
                    kind: ProgramAssemblyErrorKind::UnexpectedArgument {
                        got: argument,
                        expected: ArgumentRequirement::Instruction,
                    },
                });
            };

            let next_argument = next_argument();

            let Err(ProgramAssemblyError {
                kind: ProgramAssemblyErrorKind::InvalidArgument(ParseArgumentError::OutOfTokens),
                ..
            }) = next_argument
            else {
                return Err(ProgramAssemblyError {
                    line: index,
                    kind: ProgramAssemblyErrorKind::TooManyArguments {
                        got: next_argument?,
                    },
                });
            };

            return Ok(ParseInstructionResult::Label(label));
        }

        let instruction_kind =
            instruction_code
                .parse::<InstructionKind>()
                .map_err(|_| ProgramAssemblyError {
                    line: index,
                    kind: ProgramAssemblyErrorKind::NoSuchOperation {
                        got: instruction_code,
                    },
                })?;

        let mut arguments = array::from_fn(|_| ArgumentIntermediate::Finished(Argument::Empty));
        let mut previous_argument: Option<ArgumentIntermediate> = None;

        for (i, requirement) in instruction_kind
            .get_properties()
            .arguments
            .into_iter()
            .enumerate()
        {
            if matches!(requirement, ArgumentRequirement::Empty) {
                continue;
            }

            let argument = if let Some(argument) = previous_argument {
                if argument.matches_requirement(requirement) {
                    previous_argument = None;

                    argument
                } else if requirement.allows_empty() {
                    continue;
                } else {
                    return Err(ProgramAssemblyError {
                        line: index,
                        kind: ProgramAssemblyErrorKind::UnexpectedArgument {
                            got: argument,
                            expected: requirement,
                        },
                    });
                }
            } else {
                let argument = next_argument()?;

                if argument.matches_requirement(requirement) {
                    argument
                } else if requirement.allows_empty() {
                    previous_argument = Some(argument);
                    continue;
                } else {
                    return Err(ProgramAssemblyError {
                        line: index,
                        kind: ProgramAssemblyErrorKind::UnexpectedArgument {
                            got: argument,
                            expected: requirement,
                        },
                    });
                }
            };

            if let ArgumentIntermediate::Finished(Argument::Number(NumberSource::Register(
                register,
            ))) = argument
            {
                if !allowed_registers[register as usize] {
                    return Err(ProgramAssemblyError {
                        line: index,
                        kind: ProgramAssemblyErrorKind::NoSuchRegister { got: register },
                    });
                }
            }

            arguments[i] = argument;
        }

        Ok(ParseInstructionResult::Instruction(Self {
            kind: instruction_kind,
            line: index,
            arguments,
        }))
    }

    pub fn complete(self) -> Option<Instruction> {
        let mut arguments = array::from_fn(|_| Argument::Empty);

        for (i, argument) in self.arguments.into_iter().enumerate() {
            arguments[i] = argument.complete()?;
        }

        Some(Instruction {
            kind: self.kind,
            line: self.line,
            arguments,
        })
    }
}

impl<'a> ArgumentIntermediate<'a> {
    pub fn pop_from_tokens(
        tokens: &mut Peekable<impl Iterator<Item = &'a str>>,
    ) -> Result<Self, ParseArgumentError<'a>> {
        let Some(token) = tokens.next() else {
            return Err(ParseArgumentError::OutOfTokens);
        };

        let argument = Self::from_token(token)?;

        let Some(&token) = tokens.peek() else {
            return Ok(argument);
        };

        let Some((index, _)) = [
            &["<"][..],
            &["="],
            &[">"],
            &["≥", ">="],
            &["≠", "!=", "/="],
            &["≤", "<="],
        ]
        .into_iter()
        .enumerate()
        .find(|&(_, values)| values.contains(&token)) else {
            return Ok(argument);
        };

        tokens.next();

        let invert = index >= 3;
        let ordering = match index % 3 {
            0 => Ordering::Less,
            1 => Ordering::Equal,
            2 => Ordering::Greater,
            _ => unreachable!(),
        };

        let Some(token) = tokens.next() else {
            return Err(ParseArgumentError::OutOfTokens);
        };

        let values = [
            argument.as_number_source()?,
            Self::from_token(token)
                .map_err(|error| match error {
                    ParseArgumentError::OutOfTokens => ParseArgumentError::InvalidComparison,
                    other => other,
                })?
                .as_number_source()?,
        ];

        Ok(ArgumentIntermediate::Finished(Argument::Comparison(
            Comparison {
                ordering,
                invert,
                values,
            },
        )))
    }

    fn from_token(token: &'a str) -> Result<Self, ParseArgumentError<'a>> {
        let start = token.chars().next().unwrap();

        match start {
            'A'..='Z' | '_' => {
                if token.len() > 1 {
                    Ok(Self::Label(token))
                } else {
                    Ok(Self::Finished(Argument::Number(NumberSource::Register(
                        computer::register_with_name(start)
                            .ok_or(ParseArgumentError::InvalidRegister { got: token })?,
                    ))))
                }
            }
            '0'..='9' | '-' => Ok(Self::Finished(Argument::Number(NumberSource::Constant(
                token
                    .parse()
                    .map_err(|error| ParseArgumentError::InvalidInteger { got: token, error })?,
            )))),
            _ => return Err(ParseArgumentError::InvalidToken { got: token }),
        }
    }

    pub fn matches_requirement(&self, requirement: ArgumentRequirement) -> bool {
        match self {
            ArgumentIntermediate::Finished(argument) => argument.matches_requirement(requirement),
            ArgumentIntermediate::Label(_) => {
                matches!(requirement, ArgumentRequirement::Instruction)
            }
        }
    }

    fn complete(self) -> Option<Argument> {
        match self {
            ArgumentIntermediate::Finished(argument) => Some(argument),
            _ => None,
        }
    }

    fn as_number_source(self) -> Result<NumberSource, ParseArgumentError<'a>> {
        match self {
            ArgumentIntermediate::Finished(argument) => match argument {
                Argument::Number(source) => Ok(source),
                _ => Err(ParseArgumentError::InvalidComparison),
            },
            ArgumentIntermediate::Label(label) => {
                Err(ParseArgumentError::InvalidLabel { got: label })
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ParseArgumentError<'a> {
    OutOfTokens,
    InvalidToken { got: &'a str },
    InvalidRegister { got: &'a str },
    InvalidInteger { got: &'a str, error: ParseIntError },
    InvalidLabel { got: &'a str },
    InvalidComparison,
}
