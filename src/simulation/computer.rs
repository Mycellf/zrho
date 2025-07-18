use std::{
    array,
    fmt::{Debug, Display},
    iter,
    ops::{Deref, DerefMut},
    slice,
};

use super::{
    instruction::{
        ArgumentValues, InstructionEvaluationInterrupt, InstructionKindMap, InstructionProperties,
    },
    integer::{AssignIntegerError, DigitInteger, Integer},
    program::Program,
};

#[derive(Clone, Debug)]
pub struct Computer {
    pub instruction_properties: InstructionKindMap<InstructionProperties>,

    pub registers: RegisterSet,
    pub maximum_digits: u8,

    pub instruction: u32,
    pub block_time: u32,
    pub tick_complete: bool,

    next_instruction: u32,

    pub runtime: u64,
    pub energy_used: u64,

    pub executed_instructions: InstructionKindMap<u8>,
    pub executed_instruction_groups: InstructionKindMap<u8>,

    pub previous_instruction: Option<(u32, ArgumentValues)>,
    pub interrupt: Option<InstructionEvaluationInterrupt>,
}

impl Computer {
    pub fn new(
        maximum_digits: u8,
        registers: RegisterSet,
        instruction_properties: InstructionKindMap<InstructionProperties>,
    ) -> Self {
        Self {
            instruction_properties,

            registers,
            maximum_digits,

            instruction: 0,
            block_time: 0,
            tick_complete: false,

            next_instruction: 0,

            runtime: 0,
            energy_used: 0,

            executed_instructions: InstructionKindMap::from_element(0),
            executed_instruction_groups: InstructionKindMap::from_element(0),

            previous_instruction: None,
            interrupt: None,
        }
    }

    pub fn reset(&mut self) {
        let mut registers = std::mem::take(&mut self.registers);
        registers.reset_to_zero();

        let instruction_properties = std::mem::take(&mut self.instruction_properties);

        *self = Computer::new(self.maximum_digits, registers, instruction_properties);
    }

    pub fn step_tick(&mut self, program: &Program) {
        while self.interrupt.is_none() {
            self.step_cycle(program);

            if self.tick_complete {
                break;
            }
        }
    }

    /// Returns the amount of ticks taken during the step
    pub fn step_instruction(&mut self, program: &Program) -> u64 {
        let mut ticks = 0;

        while self.interrupt.is_none() {
            let did_something = self.step_cycle(program);

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
    pub fn step_cycle(&mut self, program: &Program) -> bool {
        self.tick_complete = true;

        if self.interrupt.is_some() {
            return false;
        }

        if self.block_time > 0 {
            self.block_time -= 1;
        } else {
            let instruction = program.instructions.get(self.next_instruction as usize);

            if let Some(instruction) = instruction {
                let previous_instruction = self.previous_instruction.as_ref().map(
                    |&(instruction, ref argument_values)| {
                        (&program.instructions[instruction as usize], argument_values)
                    },
                );

                let properties = self.instruction_properties[instruction.kind];

                let limit = properties.calls_per_tick_limit;

                let group = instruction.group(&self.instruction_properties, previous_instruction);

                if limit.is_some_and(|limit| self.executed_instruction_groups[group] >= limit.get())
                {
                    self.end_of_tick();
                    return false;
                }

                self.executed_instructions[instruction.kind] += 1;
                self.executed_instruction_groups[group] += 1;

                match instruction.evaluate(
                    &mut self.registers,
                    &self.instruction_properties,
                    previous_instruction,
                    &mut self.next_instruction,
                    self.runtime,
                ) {
                    Ok((time, argument_values, update_previous_instruction)) => {
                        self.previous_instruction = update_previous_instruction
                            .then_some((self.instruction, argument_values));

                        if time == 0 {
                            self.tick_complete = false;
                        } else if time > 0 {
                            self.block_time = time - 1;
                        }

                        if let Some(energy_used) = self.energy_used.checked_add(1) {
                            self.energy_used = energy_used;
                        } else {
                            self.interrupt =
                                Some(InstructionEvaluationInterrupt::EnergyCounterOverflow);
                            self.previous_instruction = None;
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
        self.executed_instruction_groups = InstructionKindMap::from_element(0);

        if let Some(runtime) = self.runtime.checked_add(1) {
            self.runtime = runtime;
        } else {
            self.interrupt = Some(InstructionEvaluationInterrupt::RuntimeCounterOverflow);
            self.previous_instruction = None;
        }

        for register in self.registers.registers.iter_mut().flatten() {
            register.end_of_tick();
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

    pub fn reset_to_zero(&mut self) {
        self.buffered_writes.clear();

        for register in self.registers.iter_mut().flatten() {
            register.block_time = 0;

            match &mut register.values {
                RegisterValues::Scalar(value) => value.try_set(0).unwrap(),
                RegisterValues::Vector { values, index, .. } => {
                    for value in values {
                        value.try_set(0).unwrap();
                    }

                    *index = 0;
                }
            }
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
            let indexed_register = self
                .get_mut(array_index)
                .ok_or(RegisterAccessError::NoSuchRegister { got: array_index })?;

            indexed_register.indexed_by = Some(index);

            match &mut indexed_register.values {
                RegisterValues::Vector { index, .. } => {
                    if let Some(BlockCondition::IndexChange {
                        minimum_change,
                        block_time,
                    }) = indexed_register.block_condition
                    {
                        if index.abs_diff(value) >= minimum_change {
                            indexed_register.block_reason = Some(match value.cmp(index) {
                                std::cmp::Ordering::Less => BlockReason::IndexDecreased,
                                std::cmp::Ordering::Equal => BlockReason::IndexWrittenNoOp,
                                std::cmp::Ordering::Greater => BlockReason::IndexIncreased,
                            });

                            indexed_register.block_time = block_time;
                        }
                    }

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
    pub block_time: u32,
    pub block_reason: Option<BlockReason>,
    pub block_condition: Option<BlockCondition>,
    pub indexes_array: Option<u32>,
    pub indexed_by: Option<u32>,
    pub read_time: u32,
    pub write_time: u32,
}

#[derive(Clone, Copy, Debug)]
pub enum BlockReason {
    IndexIncreased,
    IndexWrittenNoOp,
    IndexDecreased,
}

#[derive(Clone, Copy, Debug)]
pub enum BlockCondition {
    IndexChange {
        minimum_change: u32,
        block_time: u32,
    },
}

impl Register {
    pub const DEFAULT: Self = Self {
        values: RegisterValues::Scalar(DigitInteger::DUMMY),
        block_time: 0,
        block_reason: None,
        block_condition: None,
        indexes_array: None,
        indexed_by: None,
        read_time: 0,
        write_time: 0,
    };

    pub fn end_of_tick(&mut self) {
        if self.block_time > 0 {
            self.block_time -= 1;

            if self.block_time == 0 {
                self.block_reason = None;
            }
        }
    }

    pub fn set_time_to_write(&self, write_time: &mut u32, block_time: &mut u32) {
        *write_time += self.write_time;
        *block_time = self.block_time.max(*block_time);
    }
}

impl Default for Register {
    fn default() -> Self {
        Register::DEFAULT
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const MAXIMUM_NUMBERS: usize = 19;

        match &self.values {
            RegisterValues::Scalar(value) => write!(f, "{value}")?,
            RegisterValues::Vector {
                values,
                index,
                offset,
            } => {
                let effective_index = index
                    .saturating_sub(*offset)
                    .clamp(0, values.len() as Integer - 1)
                    as usize;

                let (start, end, lower_hidden, upper_hidden) =
                    if values.len() <= MAXIMUM_NUMBERS + 2 {
                        (0, values.len(), false, false)
                    } else if effective_index <= MAXIMUM_NUMBERS / 2 {
                        (0, MAXIMUM_NUMBERS, false, true)
                    } else if effective_index >= values.len() - MAXIMUM_NUMBERS.div_ceil(2) {
                        (values.len() - MAXIMUM_NUMBERS, values.len(), true, false)
                    } else {
                        (
                            effective_index - MAXIMUM_NUMBERS / 2,
                            effective_index + MAXIMUM_NUMBERS.div_ceil(2),
                            true,
                            true,
                        )
                    };

                if lower_hidden {
                    write!(f, "[...,")?;
                } else {
                    write!(f, "    [")?;
                }

                let mut first = true;

                for (i, &value) in values.into_iter().enumerate().skip(start).take(end - start) {
                    if first {
                        first = false;
                    } else {
                        write!(f, ",")?;
                    }

                    let parsed_value = value.to_string();
                    let padding = iter::repeat_n(' ', value.num_digits() + 1 - parsed_value.len())
                        .collect::<String>();

                    if i as Integer == index.saturating_sub(*offset) {
                        write!(f, "{padding}>{parsed_value}")?;
                    } else {
                        write!(f, "{padding} {parsed_value}")?;
                    }
                }

                if upper_hidden {
                    write!(f, ", ...")?;
                };

                write!(f, "][{index}]")?;
            }
        };

        if let Some(array) = self.indexes_array {
            write!(f, " → {}", name_of_register(array).unwrap())?;
        }

        if self.block_time > 0 {
            write!(
                f,
                "\n(waiting for {} tick{})",
                self.block_time,
                if self.block_time == 1 { "" } else { "s" }
            )?;
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
        offset: Integer,
    },
}

impl RegisterValues {
    pub fn value(&self) -> Result<&DigitInteger, RegisterAccessError> {
        match self {
            RegisterValues::Scalar(value) => Ok(value),
            RegisterValues::Vector {
                values,
                index,
                offset,
            } => values
                .get(usize::try_from(index.saturating_sub(*offset)).map_err(|_| {
                    RegisterAccessError::IndexTooSmall {
                        got: *index,
                        minimum: *offset,
                    }
                })?)
                .ok_or_else(|| RegisterAccessError::IndexTooBig {
                    got: *index,
                    maximum: values.len() as Integer - 1 - offset,
                }),
        }
    }

    pub fn value_mut(&mut self) -> Result<&mut DigitInteger, RegisterAccessError> {
        match self {
            RegisterValues::Scalar(value) => Ok(value),
            RegisterValues::Vector {
                values,
                index,
                offset,
            } => {
                let length = values.len();

                values
                    .get_mut(usize::try_from(index.saturating_sub(*offset)).map_err(|_| {
                        RegisterAccessError::IndexTooSmall {
                            got: *index,
                            minimum: *offset,
                        }
                    })?)
                    .ok_or_else(|| RegisterAccessError::IndexTooBig {
                        got: *index,
                        maximum: length as Integer - 1 - *offset,
                    })
            }
        }
    }

    pub fn all_values(&self) -> &[DigitInteger] {
        match self {
            RegisterValues::Scalar(value) => slice::from_ref(value),
            RegisterValues::Vector { values, .. } => values,
        }
    }

    pub fn all_values_mut(&mut self) -> &mut [DigitInteger] {
        match self {
            RegisterValues::Scalar(value) => slice::from_mut(value),
            RegisterValues::Vector { values, .. } => values,
        }
    }

    pub fn index(&self) -> i32 {
        match self {
            RegisterValues::Scalar(_) => 0,
            RegisterValues::Vector { index, .. } => *index,
        }
    }

    pub fn offset(&self) -> i32 {
        match self {
            RegisterValues::Scalar(_) => 0,
            RegisterValues::Vector { offset, .. } => *offset,
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

#[must_use]
pub const fn column_of_register(register: u32) -> usize {
    match register {
        3..8 => 2,
        8..13 => 1,
        20..26 => 0,
        _ => 3,
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
