use std::{
    array, cmp::Ordering, collections::HashMap, fmt::Display, iter::Peekable, num::ParseIntError,
};

use crate::simulation::{
    computer::Computer,
    integer::{self, AssignIntegerError, DigitInteger},
};

use super::{
    argument::{Argument, Comparison, NumberSource},
    computer,
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

    pub fn assemble_from<'a>(
        name: String,
        source_code: &'a str,
        target_computer: &Computer,
    ) -> Result<Self, Vec<ProgramAssemblyError<'a>>> {
        let mut errors = Vec::new();

        let mut instructions = Vec::new();
        let mut labels = HashMap::new();

        for (i, line) in source_code.lines().enumerate() {
            match InstructionIntermediate::from_line(line, i.try_into().unwrap(), target_computer) {
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
            match instruction.parse(&labels, target_computer) {
                Ok(instruction) => program.instructions.push(instruction),
                Err(error) => errors.push(error),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        for instruction in &program.instructions {
            for argument in &instruction.arguments {
                for source in argument.number_sources() {
                    if let Some(register) = source.as_register() {
                        if target_computer.registers.get(register).is_none() {
                            errors.push(ProgramAssemblyError {
                                line: instruction.line,
                                kind: ProgramAssemblyErrorKind::RegisterNotSupported(register),
                            });
                        }
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
    NoSuchOperation(&'a str),
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
    InvalidArgument(ParseArgumentError<'a>),
}

#[derive(Clone, Copy, Debug)]
pub enum ParseArgumentError<'a> {
    OutOfTokens,
    IncompleteComparison,
    ConstantTooBig { got: &'a str, maximum: Integer },
    ConstantTooSmall { got: &'a str, minimum: Integer },
    NoSuchLabel(&'a str),
    InvalidLabel(&'a str),
    IncorrectType,
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
        target_computer: &Computer,
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
                    _ => {
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

            return if is_label_valid(label) {
                Ok(ParseInstructionResult::Label(label))
            } else {
                Err(ProgramAssemblyError {
                    line: line_index,
                    kind: ProgramAssemblyErrorKind::InvalidArgument(
                        ParseArgumentError::InvalidLabel(label),
                    ),
                })
            };
        }

        let instruction_properties = (target_computer.instruction_properties)
            .instruction_with_name(instruction_code)
            .ok_or(ProgramAssemblyError {
                line: line_index,
                kind: ProgramAssemblyErrorKind::NoSuchOperation(instruction_code),
            })?;

        Ok(ParseInstructionResult::Instruction(Self {
            kind: instruction_properties.kind,
            line: line_index,
            arguments,
        }))
    }

    pub fn parse(
        self,
        labels: &HashMap<&str, u32>,
        target_computer: &Computer,
    ) -> Result<Instruction, ProgramAssemblyError<'a>> {
        let properties = target_computer.instruction_properties[self.kind];

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

            match argument_intermediate.as_requirement(
                requirement,
                labels,
                target_computer.maximum_digits,
            ) {
                Ok(argument) => {
                    instruction.arguments[i] = argument;
                    arguments.next();
                }
                Err(error) => {
                    return if let ParseArgumentError::IncorrectType = error {
                        Err(ProgramAssemblyError {
                            line: self.line,
                            kind: ProgramAssemblyErrorKind::UnexpectedArgument {
                                got: *argument_intermediate,
                                expected: requirement,
                            },
                        })
                    } else {
                        Err(ProgramAssemblyError {
                            line: self.line,
                            kind: ProgramAssemblyErrorKind::InvalidArgument(error),
                        })
                    };
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
            Err(ProgramAssemblyError {
                line,
                kind: ProgramAssemblyErrorKind::TooFewArguments {
                    got: length,
                    minimum,
                },
            })
        } else if length > maximum {
            Err(ProgramAssemblyError {
                line,
                kind: ProgramAssemblyErrorKind::TooManyArguments {
                    got: length,
                    maximum,
                },
            })
        } else {
            Ok(())
        }
    }
}

impl<'a> ArgumentIntermediate<'a> {
    pub fn pop_from_tokens(
        tokens: &mut Peekable<impl Iterator<Item = &'a str>>,
    ) -> Result<Self, ParseArgumentError<'a>> {
        let Some(first_value) = tokens.next() else {
            return Err(ParseArgumentError::OutOfTokens);
        };

        let Some(&comparison_token) = tokens.peek() else {
            return Ok(ArgumentIntermediate::Token(first_value));
        };

        let (ordering, invert) = match comparison_token {
            "<" => (Ordering::Less, false),
            "=" => (Ordering::Equal, false),
            ">" => (Ordering::Greater, false),
            "≥" | ">=" => (Ordering::Less, true),
            "≠" | "!=" | "/=" => (Ordering::Equal, true),
            "≤" | "<=" => (Ordering::Greater, true),
            _ => return Ok(ArgumentIntermediate::Token(first_value)),
        };

        tokens.next();

        let Some(second_value) = tokens.next() else {
            return Err(ParseArgumentError::IncompleteComparison);
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
        maximum_digits: u8,
    ) -> Result<Argument, ParseArgumentError<'a>> {
        Ok(match requirement {
            ArgumentRequirement::Constant | ArgumentRequirement::ConstantOrEmpty => {
                self.as_constant(maximum_digits)?.into()
            }
            ArgumentRequirement::RegisterWriteOnly | ArgumentRequirement::Register => {
                self.as_register()?.into()
            }
            ArgumentRequirement::ConstantOrRegister => {
                self.as_number_source(maximum_digits)?.into()
            }
            ArgumentRequirement::Comparison => self.as_comparison(maximum_digits)?.into(),
            ArgumentRequirement::AnyValue | ArgumentRequirement::AnyValueOrEmpty => {
                self.as_value(maximum_digits)?
            }
            ArgumentRequirement::Instruction => {
                let label = self.as_label()?;

                Argument::Instruction(
                    *labels
                        .get(label)
                        .ok_or(ParseArgumentError::NoSuchLabel(label))?,
                )
            }
            ArgumentRequirement::Empty => return Err(ParseArgumentError::IncorrectType),
        })
    }

    pub fn as_label(&self) -> Result<&'a str, ParseArgumentError<'a>> {
        match self {
            ArgumentIntermediate::Token(label) => {
                if is_label_valid(label) {
                    Ok(label)
                } else {
                    Err(ParseArgumentError::InvalidLabel(label))
                }
            }
            ArgumentIntermediate::Comparison { .. } => Err(ParseArgumentError::IncorrectType),
        }
    }

    pub fn as_constant(&self, maximum_digits: u8) -> Result<Integer, ParseArgumentError<'a>> {
        match self {
            ArgumentIntermediate::Token(token) => {
                let value = token
                    .parse()
                    .map_err(|error: ParseIntError| match error.kind() {
                        std::num::IntErrorKind::PosOverflow => ParseArgumentError::ConstantTooBig {
                            got: token,
                            maximum: DigitInteger::range_of_digits(maximum_digits),
                        },
                        std::num::IntErrorKind::NegOverflow => {
                            ParseArgumentError::ConstantTooSmall {
                                got: token,
                                minimum: -DigitInteger::range_of_digits(maximum_digits),
                            }
                        }
                        _ => ParseArgumentError::IncorrectType,
                    })?;

                if let Err(error) = DigitInteger::new(value, maximum_digits) {
                    return Err(match error {
                        AssignIntegerError::ValueTooBig { maximum, .. }
                        | AssignIntegerError::ValueMuchTooBig { maximum, .. } => {
                            ParseArgumentError::ConstantTooBig {
                                got: token,
                                maximum,
                            }
                        }
                        AssignIntegerError::ValueTooSmall { minimum, .. }
                        | AssignIntegerError::ValueMuchTooSmall { minimum, .. } => {
                            ParseArgumentError::ConstantTooSmall {
                                got: token,
                                minimum,
                            }
                        }
                        AssignIntegerError::NumDigitsNotSupported => {
                            panic!("maximum_digits should be supported")
                        }
                    });
                };

                Ok(value)
            }
            ArgumentIntermediate::Comparison { .. } => Err(ParseArgumentError::IncorrectType),
        }
    }

    pub fn as_register(&self) -> Result<u32, ParseArgumentError<'a>> {
        match self {
            ArgumentIntermediate::Token(value) => {
                if value.len() == 1 {
                    computer::register_with_name(value.chars().next().unwrap())
                        .ok_or(ParseArgumentError::IncorrectType)
                } else {
                    Err(ParseArgumentError::IncorrectType)
                }
            }
            ArgumentIntermediate::Comparison { .. } => Err(ParseArgumentError::IncorrectType),
        }
    }

    pub fn as_number_source(
        &self,
        maximum_digits: u8,
    ) -> Result<NumberSource, ParseArgumentError<'a>> {
        let as_constant_error = match self.as_constant(maximum_digits) {
            Ok(constant) => return Ok(NumberSource::Constant(constant)),
            Err(error) => error,
        };

        let as_register_error = match self.as_register() {
            Ok(register) => return Ok(NumberSource::Register(register)),
            Err(error) => error,
        };

        Err(
            if matches!(as_constant_error, ParseArgumentError::IncorrectType) {
                as_register_error
            } else {
                as_constant_error
            },
        )
    }

    pub fn as_comparison(&self, maximum_digits: u8) -> Result<Comparison, ParseArgumentError<'a>> {
        match self {
            ArgumentIntermediate::Token(_) => Err(ParseArgumentError::IncorrectType),
            ArgumentIntermediate::Comparison {
                ordering,
                invert,
                values: [lhs, rhs],
            } => Ok(Comparison {
                ordering: *ordering,
                invert: *invert,
                values: [
                    ArgumentIntermediate::Token(lhs).as_number_source(maximum_digits)?,
                    ArgumentIntermediate::Token(rhs).as_number_source(maximum_digits)?,
                ],
            }),
        }
    }

    pub fn as_value(&self, maximum_digits: u8) -> Result<Argument, ParseArgumentError<'a>> {
        let as_source_error = match self.as_number_source(maximum_digits) {
            Ok(source) => return Ok(source.into()),
            Err(error) => error,
        };

        let as_comparison_error = match self.as_comparison(maximum_digits) {
            Ok(comparison) => return Ok(comparison.into()),
            Err(error) => error,
        };

        Err(
            if matches!(as_source_error, ParseArgumentError::IncorrectType) {
                as_comparison_error
            } else {
                as_source_error
            },
        )
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
            ProgramAssemblyErrorKind::NoSuchOperation(operation) => {
                write!(f, "No such operation \"{operation}\"")
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
            ProgramAssemblyErrorKind::InvalidArgument(error) => match error {
                ParseArgumentError::OutOfTokens => {
                    write!(f, "Ran out of tokens when parsing arguments")
                }
                ParseArgumentError::IncompleteComparison => write!(
                    f,
                    "A constant or register must follow a comparison operator"
                ),
                ParseArgumentError::ConstantTooBig { got, maximum } => {
                    integer::format_overflow_error(f, got, *maximum)
                }
                ParseArgumentError::ConstantTooSmall { got, minimum } => {
                    integer::format_underflow_error(f, got, *minimum)
                }
                ParseArgumentError::NoSuchLabel(label) => write!(f, "No such label \"{label}\""),
                ParseArgumentError::InvalidLabel(label) => write!(
                    f,
                    "Invalid label \"{label}\", must contain only _, -, letters, and numbers"
                ),
                ParseArgumentError::IncorrectType => write!(f, "(internal error) Invalid argument"),
            },
        }
    }
}

fn is_label_valid(label: &str) -> bool {
    for character in label.chars() {
        if !['_', '-'].contains(&character) && !character.is_ascii_alphanumeric() {
            return false;
        }
    }

    true
}
