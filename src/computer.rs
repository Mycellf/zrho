use std::{
    array,
    ops::{Deref, DerefMut},
};

use crate::{
    instruction::Instruction,
    integer::{DigitInteger, Integer},
};

#[derive(Clone, Debug)]
pub struct Computer {
    pub loaded_program: Program,

    pub registers: RegisterSet,

    pub instruction: u32,
    pub block_time: u32,
}

impl Computer {
    pub fn new(program: Program, registers: RegisterSet) -> Self {
        Self {
            loaded_program: program,

            registers,

            instruction: 0,
            block_time: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.block_time > 0 {
            self.block_time -= 1;
        } else {
            self.instruction += 1;

            todo!();
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
}

#[derive(Clone, Debug)]
pub struct RegisterSet {
    pub registers: [Option<Register>; NUM_REGISTERS],
}

impl RegisterSet {
    #[must_use]
    pub fn new() -> Self {
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

        Ok(std::mem::replace(register_entry, Some(register)))
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
    pub fn get_mut(&mut self, index: u32) -> Option<&mut Register> {
        self.registers.get_mut(index as usize)?.as_mut()
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
    pub read_block: u32,
    pub write_time: u32,
    pub write_block: u32,
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
    #[must_use]
    pub fn value(&self) -> Result<&DigitInteger, RegisterReadError> {
        match self {
            RegisterValues::Scalar(value) => Ok(value),
            RegisterValues::Vector { values, index } => values
                .get(
                    usize::try_from(*index).map_err(|_| RegisterReadError::IndexTooSmall {
                        got: *index,
                        minimum: 0,
                    })?,
                )
                .ok_or_else(|| RegisterReadError::IndexTooBig {
                    got: *index,
                    maximum: values.len() as i32 - 1,
                }),
        }
    }

    #[must_use]
    pub fn value_mut(&mut self) -> Result<&mut DigitInteger, RegisterReadError> {
        match self {
            RegisterValues::Scalar(value) => Ok(value),
            RegisterValues::Vector { values, index } => {
                let length = values.len();

                values
                    .get_mut(usize::try_from(*index).map_err(|_| {
                        RegisterReadError::IndexTooSmall {
                            got: *index,
                            minimum: 0,
                        }
                    })?)
                    .ok_or_else(|| RegisterReadError::IndexTooBig {
                        got: *index,
                        maximum: length as i32 - 1,
                    })
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RegisterReadError {
    IndexTooBig { got: Integer, maximum: Integer },
    IndexTooSmall { got: Integer, minimum: Integer },
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
