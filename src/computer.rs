use std::{
    array,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

use crate::{
    instruction::{ArgumentValues, InstructionEvaluationInterrupt, InstructionKindMap},
    integer::{AssignIntegerError, DigitInteger, Integer},
    program::Program,
};

#[derive(Clone, Debug)]
pub struct Computer {
    pub loaded_program: Program,

    pub registers: RegisterSet,

    pub instruction: u32,
    pub block_time: u32,
    pub tick_complete: bool,

    next_instruction: u32,

    pub runtime: u64,

    executed_instructions: InstructionKindMap<u8>,

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
            tick_complete: false,

            next_instruction: 0,

            runtime: 0,

            executed_instructions: InstructionKindMap::from_element(0),

            previous_instruction: None,
            interrupt: None,
        }
    }

    pub fn step_tick(&mut self) {
        loop {
            self.step_cycle();

            if self.tick_complete {
                break;
            }
        }
    }

    /// Returns the amount of ticks taken by the instruction
    pub fn step_instruction(&mut self) -> u64 {
        let mut ticks = 0;

        loop {
            let did_something = self.step_cycle();

            if self.tick_complete {
                ticks += 1;
            }

            if did_something && self.block_time == 0 {
                break;
            }
        }

        ticks
    }

    /// Returns whether or not there was any operation run (includes time spent blocking).
    pub fn step_cycle(&mut self) -> bool {
        self.tick_complete = true;

        if self.interrupt.is_some() {
            return false;
        }

        if self.block_time > 0 {
            self.block_time -= 1;
        } else {
            let instruction = self
                .loaded_program
                .instructions
                .get(self.next_instruction as usize);

            if let Some(instruction) = instruction {
                let properties = instruction.kind.get_properties();

                let limit = properties.calls_per_tick_limit;

                let group = properties.group();

                if limit.is_some_and(|limit| self.executed_instructions[group] >= limit.get()) {
                    self.end_of_tick();
                    return false;
                }

                self.executed_instructions[group] += 1;

                match instruction.evaluate(
                    &mut self.registers,
                    self.previous_instruction.as_ref().map(
                        |&(instruction, ref argument_values)| {
                            (
                                &self.loaded_program.instructions[instruction as usize],
                                argument_values,
                            )
                        },
                    ),
                    &mut self.next_instruction,
                ) {
                    Ok((time, argument_values, update_previous_instruction)) => {
                        self.previous_instruction = update_previous_instruction
                            .then_some((self.instruction, argument_values));

                        if time == 0 {
                            self.tick_complete = false;
                        } else if time > 0 {
                            self.block_time = time - 1;
                        }
                    }
                    Err(interrupt) => {
                        self.interrupt = Some(interrupt);
                        self.previous_instruction = None;
                    }
                }
            } else {
                self.interrupt = Some(InstructionEvaluationInterrupt::ProgramComplete);
                self.previous_instruction = None;
            }
        }

        if self.block_time == 0 {
            self.instruction = self.next_instruction;
            self.registers.apply_buffered_writes();
        }

        if self.tick_complete {
            self.end_of_tick();
        }

        true
    }

    fn end_of_tick(&mut self) {
        self.executed_instructions = InstructionKindMap::from_element(0);

        if let Some(runtime) = self.runtime.checked_add(1) {
            self.runtime = runtime;
        } else {
            self.interrupt = Some(InstructionEvaluationInterrupt::RuntimeCounterOverflow);
        }
    }
}

#[derive(Clone, Debug)]
pub struct RegisterSet {
    pub registers: Box<[Option<Register>; NUM_REGISTERS]>,
    buffered_writes: Vec<(u32, Integer)>,
}

impl RegisterSet {
    #[must_use]
    pub fn new_empty() -> Self {
        Self {
            registers: Box::new(array::from_fn(|_| None)),
            buffered_writes: Vec::new(),
        }
    }

    pub fn apply_buffered_writes(&mut self) {
        let mut buffered_writes = std::mem::take(&mut self.buffered_writes);

        for (register, value) in buffered_writes.drain(..) {
            self.write(register, value).unwrap();
        }

        self.buffered_writes = buffered_writes;
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
    pub fn with_register(mut self, register_name: char, register: Register) -> Self {
        self.add_register(register_with_name(register_name).unwrap(), register)
            .unwrap();
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

    pub fn buffered_write(
        &mut self,
        index: u32,
        value: Integer,
    ) -> Result<(), RegisterAccessError> {
        let register = self
            .get(index)
            .ok_or(RegisterAccessError::NoSuchRegister { got: index })?;
        register
            .value()?
            .is_valid(value)
            .map_err(|error| RegisterAccessError::InvalidAssignment { error })?;

        self.buffered_writes.push((index, value));

        Ok(())
    }

    pub fn write(&mut self, index: u32, value: Integer) -> Result<(), RegisterAccessError> {
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

        Ok(())
    }
}

impl Display for RegisterSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, register) in self.registers.iter().enumerate() {
            let Some(register) = register else {
                continue;
            };

            let name = name_of_register(i as u32).unwrap();

            writeln!(f, "{name}: {register}")?;
        }

        Ok(())
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

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.values {
            RegisterValues::Scalar(value) => write!(f, "{value}")?,
            RegisterValues::Vector { values, index } => {
                write!(f, "[")?;

                for (i, &value) in values.into_iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{value}")?;
                }

                write!(f, "][{index}]")?;
            }
        };

        if let Some(array) = self.indexes_array {
            write!(f, " → {}", name_of_register(array).unwrap())?;
        }

        Ok(())
    }
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

#[derive(Clone, Copy, Debug)]
pub struct RegisterMap<T>(pub [T; 26]);

impl<T> RegisterMap<T> {
    pub fn from_element(element: T) -> Self
    where
        T: Clone,
    {
        Self(array::from_fn(|_| element.clone()))
    }

    pub fn with_value(mut self, register_name: char, value: T) -> Self {
        let index = register_with_name(register_name).unwrap();

        self[index as usize] = value;

        self
    }
}

impl<T> Deref for RegisterMap<T> {
    type Target = [T; 26];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for RegisterMap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
