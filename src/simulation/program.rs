use std::{array, cmp::Ordering, collections::HashMap, fmt::Display, iter::Peekable};

use crate::simulation::{
    argument::{Argument, Comparison, NumberSource},
    computer::{self, RegisterMap},
    instruction::{ArgumentRequirement, Instruction, InstructionKind},
    integer::Integer,
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

        let mut instructions = Vec::new();
        let mut labels = HashMap::new();

        for (i, line) in source_code.lines().enumerate() {
            match InstructionIntermediate::from_line(line, i.try_into().unwrap()) {
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

        let mut program = Self::new_empty(name);

        for instruction in instructions {
            match instruction.parse(&labels) {
                Ok(instruction) => program.instructions.push(instruction),
                Err(error) => errors.push(error),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        for instruction in &program.instructions {
            for argument in instruction.arguments {
                if let Some(register) = argument.as_register() {
                    if !allowed_registers[register as usize] {
                        errors.push(ProgramAssemblyError {
                            line: instruction.line,
                            kind: ProgramAssemblyErrorKind::RegisterNotSupported(register),
                        });
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(program)
        } else {
            Err(errors)
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProgramAssemblyError<'a> {
    pub line: u32,
    pub kind: ProgramAssemblyErrorKind<'a>,
}

#[derive(Clone, Debug)]
pub enum ProgramAssemblyErrorKind<'a> {
    RegisterNotSupported(u32),
    NoSuchLabel(&'a str),
    NoSuchOperation(&'a str),
    InvalidArgument(ParseArgumentError),
    UnexpectedArgument {
        got: ArgumentIntermediate<'a>,
        expected: ArgumentRequirement,
    },
    TooManyArguments {
        got: usize,
        maximum: usize,
    },
    TooFewArguments {
        got: usize,
        minimum: usize,
    },
}

#[derive(Clone, Debug)]
struct InstructionIntermediate<'a> {
    kind: InstructionKind,
    line: u32,
    arguments: Vec<ArgumentIntermediate<'a>>,
}

enum ParseInstructionResult<'a> {
    Instruction(InstructionIntermediate<'a>),
    Label(&'a str),
    Empty,
}

#[derive(Clone, Copy, Debug)]
pub enum ArgumentIntermediate<'a> {
    Token(&'a str),
    Comparison {
        ordering: Ordering,
        invert: bool,
        values: [&'a str; 2],
    },
}

impl<'a> InstructionIntermediate<'a> {
    fn from_line(
        source_line: &'a str,
        line_index: u32,
    ) -> Result<ParseInstructionResult<'a>, ProgramAssemblyError<'a>> {
        let line = source_line
            .split_once(COMMENT_SEPARATOR)
            .map(|(line, _)| line)
            .unwrap_or(source_line);

        let mut tokens = line.split_whitespace().peekable();

        let Some(instruction_code) = tokens.next() else {
            return Ok(ParseInstructionResult::Empty);
        };

        let mut arguments = Vec::new();

        loop {
            match ArgumentIntermediate::pop_from_tokens(&mut tokens) {
                Ok(argument) => arguments.push(argument),
                Err(error) => match error {
                    ParseArgumentError::OutOfTokens => break,
                    ParseArgumentError::InvalidComparison => {
                        return Err(ProgramAssemblyError {
                            line: line_index,
                            kind: ProgramAssemblyErrorKind::InvalidArgument(error),
                        });
                    }
                },
            }
        }

        if instruction_code == LABEL_PSEUDO_INSTRUCTION {
            Self::check_argument_length(arguments.len(), 1, 1, line_index)?;

            let argument = arguments[0];

            let ArgumentIntermediate::Token(label) = argument else {
                return Err(ProgramAssemblyError {
                    line: line_index,
                    kind: ProgramAssemblyErrorKind::UnexpectedArgument {
                        got: argument,
                        expected: ArgumentRequirement::Instruction,
                    },
                });
            };

            return Ok(ParseInstructionResult::Label(label));
        }

        let instruction_kind =
            instruction_code
                .parse::<InstructionKind>()
                .map_err(|_| ProgramAssemblyError {
                    line: line_index,
                    kind: ProgramAssemblyErrorKind::NoSuchOperation(instruction_code),
                })?;

        Ok(ParseInstructionResult::Instruction(Self {
            kind: instruction_kind,
            line: line_index,
            arguments,
        }))
    }

    pub fn parse(
        self,
        labels: &HashMap<&str, u32>,
    ) -> Result<Instruction, ProgramAssemblyError<'a>> {
        let properties = self.kind.get_properties();

        let min_arguments = properties.minimum_arguments();
        let max_arguments = properties.maximum_arguments();

        Self::check_argument_length(
            self.arguments.len(),
            min_arguments,
            max_arguments,
            self.line,
        )?;

        let mut skipped = 0;

        let mut arguments = self.arguments.iter().peekable();

        let mut instruction = Instruction {
            kind: self.kind,
            line: self.line,
            arguments: array::from_fn(|_| Argument::Empty),
        };

        for (i, requirement) in properties.arguments.into_iter().enumerate() {
            if requirement == ArgumentRequirement::Empty
                || requirement.allows_empty() && max_arguments > skipped + self.arguments.len()
            {
                skipped += 1;
                continue;
            }

            let argument_intermediate = *arguments.peek().unwrap();

            match argument_intermediate.as_requirement(requirement, labels) {
                Some(argument) => {
                    instruction.arguments[i] = argument;
                    arguments.next();
                }
                None => {
                    if let (ArgumentRequirement::Instruction, ArgumentIntermediate::Token(label)) =
                        (requirement, argument_intermediate)
                    {
                        return Err(ProgramAssemblyError {
                            line: self.line,
                            kind: ProgramAssemblyErrorKind::NoSuchLabel(*label),
                        });
                    }

                    return Err(ProgramAssemblyError {
                        line: self.line,
                        kind: ProgramAssemblyErrorKind::UnexpectedArgument {
                            got: *argument_intermediate,
                            expected: requirement,
                        },
                    });
                }
            }
        }

        Ok(instruction)
    }

    fn check_argument_length(
        length: usize,
        minimum: usize,
        maximum: usize,
        line: u32,
    ) -> Result<(), ProgramAssemblyError<'a>> {
        if length < minimum {
            return Err(ProgramAssemblyError {
                line: line,
                kind: ProgramAssemblyErrorKind::TooFewArguments {
                    got: length,
                    minimum,
                },
            });
        } else if length > maximum {
            return Err(ProgramAssemblyError {
                line: line,
                kind: ProgramAssemblyErrorKind::TooManyArguments {
                    got: length,
                    maximum,
                },
            });
        } else {
            Ok(())
        }
    }
}

impl<'a> ArgumentIntermediate<'a> {
    pub fn pop_from_tokens(
        tokens: &mut Peekable<impl Iterator<Item = &'a str>>,
    ) -> Result<Self, ParseArgumentError> {
        let Some(first_value) = tokens.next() else {
            return Err(ParseArgumentError::OutOfTokens);
        };

        let Some(&comparison_token) = tokens.peek() else {
            return Ok(ArgumentIntermediate::Token(first_value));
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
        .find(|&(_, values)| values.contains(&comparison_token)) else {
            return Ok(ArgumentIntermediate::Token(first_value));
        };

        tokens.next();

        let invert = index >= 3;
        let ordering = match index % 3 {
            0 => Ordering::Less,
            1 => Ordering::Equal,
            2 => Ordering::Greater,
            _ => unreachable!(),
        };

        let Some(second_value) = tokens.next() else {
            return Err(ParseArgumentError::InvalidComparison);
        };

        Ok(ArgumentIntermediate::Comparison {
            ordering,
            invert,
            values: [first_value, second_value],
        })
    }

    pub fn as_requirement(
        &self,
        requirement: ArgumentRequirement,
        labels: &HashMap<&str, u32>,
    ) -> Option<Argument> {
        Some(match requirement {
            ArgumentRequirement::Constant | ArgumentRequirement::ConstantOrEmpty => {
                self.as_constant()?.into()
            }
            ArgumentRequirement::RegisterWriteOnly | ArgumentRequirement::Register => {
                self.as_register()?.into()
            }
            ArgumentRequirement::ConstantOrRegister => self.as_number_source()?.into(),
            ArgumentRequirement::Comparison => self.as_comparison()?.into(),
            ArgumentRequirement::AnyValue | ArgumentRequirement::AnyValueOrEmpty => {
                self.as_value()?
            }
            ArgumentRequirement::Instruction => {
                let label = self.as_label()?;

                Argument::Instruction(*labels.get(label)?)
            }
            ArgumentRequirement::Empty => return None,
        })
    }

    pub fn as_label(&self) -> Option<&'a str> {
        match self {
            ArgumentIntermediate::Token(label) => Some(label),
            ArgumentIntermediate::Comparison { .. } => None,
        }
    }

    pub fn as_constant(&self) -> Option<Integer> {
        match self {
            ArgumentIntermediate::Token(value) => value.parse().ok(),
            ArgumentIntermediate::Comparison { .. } => None,
        }
    }

    pub fn as_register(&self) -> Option<u32> {
        match self {
            ArgumentIntermediate::Token(value) => {
                if value.len() == 1 {
                    computer::register_with_name(value.chars().next().unwrap())
                } else {
                    None
                }
            }
            ArgumentIntermediate::Comparison { .. } => None,
        }
    }

    pub fn as_number_source(&self) -> Option<NumberSource> {
        if let Some(constant) = self.as_constant() {
            return Some(NumberSource::Constant(constant));
        }

        if let Some(register) = self.as_register() {
            return Some(NumberSource::Register(register));
        }

        None
    }

    pub fn as_comparison(&self) -> Option<Comparison> {
        match self {
            ArgumentIntermediate::Token(_) => None,
            ArgumentIntermediate::Comparison {
                ordering,
                invert,
                values,
            } => match values.map(|value| ArgumentIntermediate::Token(value).as_number_source()) {
                [Some(lhs), Some(rhs)] => Some(Comparison {
                    ordering: *ordering,
                    invert: *invert,
                    values: [lhs, rhs],
                }),
                _ => None,
            },
        }
    }

    pub fn as_value(&self) -> Option<Argument> {
        if let Some(source) = self.as_number_source() {
            return Some(source.into());
        }

        self.as_comparison().map(Comparison::into)
    }
}

impl Display for ArgumentIntermediate<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgumentIntermediate::Token(token) => write!(f, "{token}"),
            ArgumentIntermediate::Comparison {
                ordering,
                invert,
                values,
            } => {
                const SYMBOLS: [char; 6] = ['<', '=', '>', '≥', '≠', '≤'];
                let index = (*ordering as isize + 1) as usize + *invert as usize * 3;

                write!(
                    f,
                    "{lhs} {comparison} {rhs}",
                    lhs = values[0],
                    rhs = values[1],
                    comparison = SYMBOLS[index],
                )
            }
        }
    }
}

impl Display for ProgramAssemblyError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Line {line}: {error}",
            line = self.line,
            error = self.kind,
        )
    }
}

impl Display for ProgramAssemblyErrorKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgramAssemblyErrorKind::RegisterNotSupported(register) => {
                write!(
                    f,
                    "\"{name}\" register not supported on this machine",
                    name = computer::name_of_register(*register).unwrap(),
                )
            }
            ProgramAssemblyErrorKind::NoSuchLabel(label) => write!(f, "No such label \"{label}\""),
            ProgramAssemblyErrorKind::NoSuchOperation(operation) => {
                write!(f, "No such operation \"{operation}\"")
            }
            ProgramAssemblyErrorKind::InvalidArgument(ParseArgumentError::InvalidComparison) => {
                write!(
                    f,
                    "A constant or register must follow a comparison operator"
                )
            }
            ProgramAssemblyErrorKind::InvalidArgument(ParseArgumentError::OutOfTokens) => {
                write!(f, "Ran out of tokens when parsing arguments")
            }
            ProgramAssemblyErrorKind::UnexpectedArgument { got, expected } => {
                write!(f, "Got \"{got}\", expected {expected}")
            }
            ProgramAssemblyErrorKind::TooManyArguments { got, maximum } => {
                write!(f, "Too many arguments (got {got}, maximum {maximum})")
            }
            ProgramAssemblyErrorKind::TooFewArguments { got, minimum } => {
                write!(f, "Not enough arguments (got {got}, minimum {minimum})")
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ParseArgumentError {
    OutOfTokens,
    InvalidComparison,
}

#[derive(Clone, Copy, Debug)]
pub struct ParseComparisonError {
    pub lhs_invalid: bool,
    pub rhs_invalid: bool,
}
