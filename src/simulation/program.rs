use std::{
    array, cmp::Ordering, collections::HashMap, fmt::Display, iter::Peekable, num::ParseIntError,
};

use super::{
    argument::{Argument, Comparison, NumberSource},
    computer::{self, Computer},
    instruction::{ArgumentRequirement, Instruction, InstructionKind},
    integer::{self, AssignIntegerError, DigitInteger, Integer},
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
    ) -> Result<Self, Vec<ProgramAssemblyError>> {
        let mut errors = Vec::new();

        let mut instructions = Vec::new();
        let mut labels = HashMap::new();

        let mut duplicate_labels = HashMap::<&str, Vec<LabelIndex>>::new();

        for (i, line) in source_code.lines().enumerate() {
            let i = i.try_into().unwrap();

            match InstructionIntermediate::from_line(line, i, target_computer) {
                Ok(instruction_result) => match instruction_result {
                    ParseInstructionResult::Instruction(instruction) => {
                        instructions.push(instruction);
                    }
                    ParseInstructionResult::Label(label) => {
                        let label_position = LabelIndex {
                            index: instructions.len().try_into().unwrap(),
                            line: i,
                        };

                        if let Some(duplicate_label) = labels.insert(label, label_position) {
                            if let Some(indecies) = duplicate_labels.get_mut(label) {
                                // It has already been checked
                                indecies.push(label_position);
                            } else {
                                duplicate_labels
                                    .insert(label, vec![duplicate_label, label_position]);
                            }
                        }
                    }
                    ParseInstructionResult::Empty => (),
                },
                Err(error) => errors.push(error),
            }
        }

        if !duplicate_labels.is_empty() {
            let mut duplicate_labels = duplicate_labels.into_iter().collect::<Vec<_>>();
            duplicate_labels.sort_unstable_by_key(|&(label, _)| label);

            for (label, indecies) in duplicate_labels {
                errors.push(ProgramAssemblyError {
                    lines: indecies
                        .into_iter()
                        .map(|LabelIndex { line, .. }| line)
                        .collect(),
                    kind: ProgramAssemblyErrorKind::DuplicateLabel(label.to_owned()),
                });
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
                                lines: vec![instruction.line],
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
pub struct ProgramAssemblyError {
    pub lines: Vec<u32>,
    pub kind: ProgramAssemblyErrorKind,
}

#[derive(Clone, Debug)]
pub enum ProgramAssemblyErrorKind {
    RegisterNotSupported(u32),
    NoSuchOperation(String),
    DuplicateLabel(String),
    UnexpectedArgument {
        got: OwnedArgumentIntermediate,
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
    InvalidArgument(ParseArgumentError),
}

#[derive(Clone, Debug)]
pub enum ParseArgumentError {
    OutOfTokens,
    IncompleteComparison,
    ConstantTooBig { got: String, maximum: Integer },
    ConstantTooSmall { got: String, minimum: Integer },
    NoSuchLabel(String),
    InvalidLabel(String),
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

impl<'a> ArgumentIntermediate<'a> {
    pub fn to_owned(&self) -> OwnedArgumentIntermediate {
        match *self {
            ArgumentIntermediate::Token(slice) => {
                OwnedArgumentIntermediate::Token(slice.to_owned())
            }
            ArgumentIntermediate::Comparison {
                ordering,
                invert,
                values,
            } => OwnedArgumentIntermediate::Comparison {
                ordering,
                invert,
                values: values.map(|slice| slice.to_owned()),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub enum OwnedArgumentIntermediate {
    Token(String),
    Comparison {
        ordering: Ordering,
        invert: bool,
        values: [String; 2],
    },
}

#[derive(Clone, Copy, Debug)]
pub struct LabelIndex {
    index: u32,
    line: u32,
}

impl<'a> InstructionIntermediate<'a> {
    fn from_line(
        source_line: &'a str,
        line_index: u32,
        target_computer: &Computer,
    ) -> Result<ParseInstructionResult<'a>, ProgramAssemblyError> {
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
                            lines: vec![line_index],
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
                    lines: vec![line_index],
                    kind: ProgramAssemblyErrorKind::UnexpectedArgument {
                        got: argument.to_owned(),
                        expected: ArgumentRequirement::Instruction,
                    },
                });
            };

            return if is_label_valid(label) {
                Ok(ParseInstructionResult::Label(label))
            } else {
                Err(ProgramAssemblyError {
                    lines: vec![line_index],
                    kind: ProgramAssemblyErrorKind::InvalidArgument(
                        ParseArgumentError::InvalidLabel(label.to_owned()),
                    ),
                })
            };
        }

        let instruction_properties = (target_computer.instruction_properties)
            .instruction_with_name(instruction_code)
            .ok_or(ProgramAssemblyError {
                lines: vec![line_index],
                kind: ProgramAssemblyErrorKind::NoSuchOperation(instruction_code.to_owned()),
            })?;

        Ok(ParseInstructionResult::Instruction(Self {
            kind: instruction_properties.kind,
            line: line_index,
            arguments,
        }))
    }

    pub fn parse(
        self,
        labels: &HashMap<&str, LabelIndex>,
        target_computer: &Computer,
    ) -> Result<Instruction, ProgramAssemblyError> {
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
                            lines: vec![self.line],
                            kind: ProgramAssemblyErrorKind::UnexpectedArgument {
                                got: argument_intermediate.to_owned(),
                                expected: requirement,
                            },
                        })
                    } else {
                        Err(ProgramAssemblyError {
                            lines: vec![self.line],
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
    ) -> Result<(), ProgramAssemblyError> {
        if length < minimum {
            Err(ProgramAssemblyError {
                lines: vec![line],
                kind: ProgramAssemblyErrorKind::TooFewArguments {
                    got: length,
                    minimum,
                },
            })
        } else if length > maximum {
            Err(ProgramAssemblyError {
                lines: vec![line],
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
    ) -> Result<Self, ParseArgumentError> {
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
        labels: &HashMap<&str, LabelIndex>,
        maximum_digits: u8,
    ) -> Result<Argument, ParseArgumentError> {
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
                    labels
                        .get(label)
                        .ok_or(ParseArgumentError::NoSuchLabel(label.to_owned()))?
                        .index,
                )
            }
            ArgumentRequirement::Empty => return Err(ParseArgumentError::IncorrectType),
        })
    }

    pub fn as_label(&self) -> Result<&'a str, ParseArgumentError> {
        match *self {
            ArgumentIntermediate::Token(label) => {
                if is_label_valid(label) {
                    Ok(label)
                } else {
                    Err(ParseArgumentError::InvalidLabel(label.to_owned()))
                }
            }
            ArgumentIntermediate::Comparison { .. } => Err(ParseArgumentError::IncorrectType),
        }
    }

    pub fn as_constant(&self, maximum_digits: u8) -> Result<Integer, ParseArgumentError> {
        match *self {
            ArgumentIntermediate::Token(token) => {
                let value = token
                    .parse()
                    .map_err(|error: ParseIntError| match error.kind() {
                        std::num::IntErrorKind::PosOverflow => ParseArgumentError::ConstantTooBig {
                            got: token.to_owned(),
                            maximum: DigitInteger::range_of_digits(maximum_digits),
                        },
                        std::num::IntErrorKind::NegOverflow => {
                            ParseArgumentError::ConstantTooSmall {
                                got: token.to_owned(),
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
                                got: token.to_owned(),
                                maximum,
                            }
                        }
                        AssignIntegerError::ValueTooSmall { minimum, .. }
                        | AssignIntegerError::ValueMuchTooSmall { minimum, .. } => {
                            ParseArgumentError::ConstantTooSmall {
                                got: token.to_owned(),
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

    pub fn as_register(&self) -> Result<u32, ParseArgumentError> {
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

    pub fn as_number_source(&self, maximum_digits: u8) -> Result<NumberSource, ParseArgumentError> {
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

    pub fn as_comparison(&self, maximum_digits: u8) -> Result<Comparison, ParseArgumentError> {
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

    pub fn as_value(&self, maximum_digits: u8) -> Result<Argument, ParseArgumentError> {
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

impl Display for OwnedArgumentIntermediate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnedArgumentIntermediate::Token(token) => write!(f, "{token}"),
            OwnedArgumentIntermediate::Comparison {
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

impl Display for ProgramAssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.lines.len() {
            0 => write!(f, "Error: {error}", error = self.kind),
            1 => write!(
                f,
                "Line {line}: {error}",
                line = self.lines[0],
                error = self.kind,
            ),
            2.. => {
                write!(f, "Lines ",)?;
                for (i, line) in self.lines.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{line}")?;
                }
                write!(f, ": {error}", error = self.kind)
            }
        }
    }
}

impl Display for ProgramAssemblyErrorKind {
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
                write!(f, "No such operation \"{operation}\" on this machine")
            }
            ProgramAssemblyErrorKind::DuplicateLabel(label) => {
                write!(f, "Duplicate label \"{label}\"")
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
