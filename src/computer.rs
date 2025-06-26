use std::{
    array,
    ops::{Deref, DerefMut},
};

use crate::{
    instruction::{ArgumentValues, Instruction, InstructionEvaluationInterrupt},
    integer::{AssignIntegerError, DigitInteger, Integer},
};

#[derive(Clone, Debug)]
pub struct Computer {
    pub loaded_program: Program,

    pub registers: RegisterSet,

    pub instruction: u32,
    pub block_time: u32,
    pub tick_complete: bool,

    pub previous_instruction: Option<(u32, ArgumentValues)>,
    pub interrupt: Option<InstructionEvaluationInterrupt>,
}

impl Computer {
    pub fn new(program: Program, registers: RegisterSet) -> Self {
        Self {
            loaded_program: program,

            registers,

            instruction: 0,
            block_time: 0,
            tick_complete: true,

            previous_instruction: None,
            interrupt: None,
        }
    }

    pub fn tick(&mut self) {
        loop {
            self.tick_partial();

            if self.tick_complete {
                break;
            }
        }
    }

    pub fn tick_partial(&mut self) {
        self.tick_complete = true;

        if self.interrupt.is_some() {
            return;
        }

        if self.block_time > 0 {
            self.block_time -= 1;
        } else {
            let previous_instruction = self.instruction;

            match self
                .loaded_program
                .instructions
                .get(self.instruction as usize)
                .map(|instruction| {
                    instruction.evaluate(
                        &mut self.registers,
                        self.previous_instruction.as_ref().map(
                            |&(instruction, ref argument_values)| {
                                (
                                    &self.loaded_program.instructions[instruction as usize],
                                    argument_values,
                                )
                            },
                        ),
                        &mut self.instruction,
                    )
                }) {
                Some(Ok((time, argument_values, update_previous_instruction))) => {
                    self.previous_instruction = update_previous_instruction
                        .then_some((previous_instruction, argument_values));

                    if time == 0 {
                        self.tick_complete = false;
                    } else if time > 0 {
                        self.block_time = time - 1;
                    }
                }
                Some(Err(interrupt)) => {
                    self.interrupt = Some(interrupt);
                    self.previous_instruction = None;
                }
                None => {
                    self.interrupt = Some(InstructionEvaluationInterrupt::ProgramComplete);
                    self.previous_instruction = None;
                }
            }
        }
    }
}

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
}

#[derive(Clone, Debug)]
pub struct RegisterSet {
    pub registers: [Option<Register>; NUM_REGISTERS],
}

impl RegisterSet {
    #[must_use]
    pub fn new_empty() -> Self {
        Self {
            registers: array::from_fn(|_| None),
        }
    }

    pub fn add_register(
        &mut self,
        index: u32,
        register: Register,
    ) -> Result<Option<Register>, CreateRegisterError> {
        let register_entry = self.registers.get_mut(index as usize).ok_or(
            CreateRegisterError::IndexOutOfBounds {
                got: index,
                maximum: NUM_REGISTERS as u32 - 1,
            },
        )?;

        Ok(register_entry.replace(register))
    }

    #[must_use]
    pub fn with_register(mut self, index: u32, register: Register) -> Self {
        self.add_register(index, register).unwrap();
        self
    }

    #[must_use]
    pub fn get(&self, index: u32) -> Option<&Register> {
        self.registers.get(index as usize)?.as_ref()
    }

    #[must_use]
    fn get_mut(&mut self, index: u32) -> Option<&mut Register> {
        self.registers.get_mut(index as usize)?.as_mut()
    }

    pub fn write(&mut self, index: u32, value: Integer) -> Result<&Register, RegisterAccessError> {
        let register = self
            .get_mut(index)
            .ok_or(RegisterAccessError::NoSuchRegister { got: index })?;
        register
            .value_mut()?
            .try_set(value)
            .map_err(|error| RegisterAccessError::InvalidAssignment { error })?;

        if let Some(array_index) = register.indexes_array {
            match &mut self
                .get_mut(array_index)
                .ok_or(RegisterAccessError::NoSuchRegister { got: array_index })?
                .values
            {
                RegisterValues::Vector { index, .. } => {
                    *index = value;
                }
                _ => return Err(RegisterAccessError::NoSuchRegister { got: array_index }),
            }
        }

        Ok(self.get(index).unwrap())
    }
}

impl Default for RegisterSet {
    fn default() -> Self {
        Self::new_empty()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CreateRegisterError {
    IndexOutOfBounds { got: u32, maximum: u32 },
}

#[derive(Clone, Debug)]
pub struct Register {
    pub values: RegisterValues,
    pub indexes_array: Option<u32>,
    pub read_time: u32,
    pub write_time: u32,
    // pub block_time: u32,
}

impl Deref for Register {
    type Target = RegisterValues;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl DerefMut for Register {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

#[derive(Clone, Debug)]
pub enum RegisterValues {
    Scalar(DigitInteger),
    Vector {
        values: Box<[DigitInteger]>,
        index: Integer,
    },
}

impl RegisterValues {
    pub fn value(&self) -> Result<&DigitInteger, RegisterAccessError> {
        match self {
            RegisterValues::Scalar(value) => Ok(value),
            RegisterValues::Vector { values, index } => values
                .get(
                    usize::try_from(*index).map_err(|_| RegisterAccessError::IndexTooSmall {
                        got: *index,
                        minimum: 0,
                    })?,
                )
                .ok_or_else(|| RegisterAccessError::IndexTooBig {
                    got: *index,
                    maximum: values.len() as i32 - 1,
                }),
        }
    }

    pub fn value_mut(&mut self) -> Result<&mut DigitInteger, RegisterAccessError> {
        match self {
            RegisterValues::Scalar(value) => Ok(value),
            RegisterValues::Vector { values, index } => {
                let length = values.len();

                values
                    .get_mut(usize::try_from(*index).map_err(|_| {
                        RegisterAccessError::IndexTooSmall {
                            got: *index,
                            minimum: 0,
                        }
                    })?)
                    .ok_or_else(|| RegisterAccessError::IndexTooBig {
                        got: *index,
                        maximum: length as i32 - 1,
                    })
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RegisterAccessError {
    IndexTooBig { got: Integer, maximum: Integer },
    IndexTooSmall { got: Integer, minimum: Integer },
    NoSuchRegister { got: u32 },
    InvalidAssignment { error: AssignIntegerError },
}

pub const NUM_REGISTERS: usize = 26;
pub const MAX_REGISTER: u32 = NUM_REGISTERS as u32;

#[must_use]
pub fn name_of_register(register: u32) -> Option<char> {
    if register < 26 {
        Some(char::try_from(register + 'A' as u32).unwrap())
    } else {
        None
    }
}

#[must_use]
pub const fn register_with_name(name: char) -> Option<u32> {
    if name >= 'A' && name <= 'Z' {
        Some(name as u32 - 'A' as u32)
    } else {
        None
    }
}
