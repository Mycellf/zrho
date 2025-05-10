use std::cmp::Ordering;

use crate::integer::DigitInteger;

pub struct ZRhoComputer {}

pub struct ZRhoProgram {
    pub instructions: Vec<InstructionEntry>,
}

pub struct RegisterPrototype {
    pub name: char,
    pub read_delay: u8,
    pub write_delay: u8,
    pub linked_to: Option<usize>,
    pub value: DigitInteger,
}

pub struct InstructionEntry {
    pub source_line: usize,
    pub instruction: Instruction,
}

pub enum Instruction {
    Set(Register, Value),

    Add(Value, Value, Register),
    Sub(Value, Value, Register),
    Mul(Value, Value, Register),
    Div(Value, Value, Register),
    Mod(Value, Value, Register),

    Cmp(Condition, Register),
    Tcp(Condition, Register),
    Fcp(Condition, Register),

    Lbl(Label),
    Jmp(Label),
    JmpCond(Condition, Label),
    Ljp(Condition, Label),
    Ujp(Condition, Label),
}

impl Instruction {
    // This is awful. There must be a better way
    pub fn new(name: &str, arguments: Vec<Argument>) -> Option<Self> {
        #[rustfmt::skip]
        let instruction = match name {
            "SET" => Self::Set(arguments.get(0)?.as_register()?, arguments.get(1)?.as_value()?),

            "ADD" => Self::Add(arguments.get(0)?.as_value()?, arguments.get(1)?.as_value()?, arguments.get(2)?.as_register()?),
            "SUB" => Self::Sub(arguments.get(0)?.as_value()?, arguments.get(1)?.as_value()?, arguments.get(2)?.as_register()?),
            "MUL" => Self::Mul(arguments.get(0)?.as_value()?, arguments.get(1)?.as_value()?, arguments.get(2)?.as_register()?),
            "DIV" => Self::Div(arguments.get(0)?.as_value()?, arguments.get(1)?.as_value()?, arguments.get(2)?.as_register()?),
            "MOD" => Self::Mod(arguments.get(0)?.as_value()?, arguments.get(1)?.as_value()?, arguments.get(2)?.as_register()?),

            "CMP" => Self::Cmp(arguments.get(0)?.as_condition()?, arguments.get(1)?.as_register()?),
            "TCP" => Self::Cmp(arguments.get(0)?.as_condition()?, arguments.get(1)?.as_register()?),
            "FCP" => Self::Cmp(arguments.get(0)?.as_condition()?, arguments.get(1)?.as_register()?),

            "LBL" => Self::Lbl(arguments.get(0)?.as_label()?),
            "JMP" => match arguments.len() {
                1 => Self::Jmp(arguments.get(0)?.as_label()?),
                2.. => Self::JmpCond(arguments.get(0)?.as_condition()?, arguments.get(1)?.as_label()?),
                _ => return None,
            },
            "LJP" => Self::Ljp(arguments.get(0)?.as_condition()?, arguments.get(1)?.as_label()?),
            "UJP" => Self::Ujp(arguments.get(0)?.as_condition()?, arguments.get(1)?.as_label()?),

            _ => return None,
        };

        if arguments.len() != instruction.num_arguments() {
            return None;
        }

        Some(instruction)
    }

    pub fn num_arguments(&self) -> usize {
        match self {
            Instruction::Set(..) => 2,

            Instruction::Add(..) => 3,
            Instruction::Sub(..) => 3,
            Instruction::Mul(..) => 3,
            Instruction::Div(..) => 3,
            Instruction::Mod(..) => 3,

            Instruction::Cmp(..) => 2,
            Instruction::Tcp(..) => 2,
            Instruction::Fcp(..) => 2,

            Instruction::Lbl(..) => 1,
            Instruction::Jmp(..) => 1,
            Instruction::JmpCond(..) => 2,
            Instruction::Ljp(..) => 2,
            Instruction::Ujp(..) => 2,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Constant(pub DigitInteger);

#[derive(Clone, Copy, Debug)]
pub struct Register(pub u32);

#[derive(Clone, Copy, Debug)]
pub enum Value {
    Constant(Constant),
    Register(Register),
}

#[derive(Clone, Copy, Debug)]
pub enum Condition {
    Direct(Value),
    Evaluate(Value, Value, ConditionKind),
}

#[derive(Clone, Copy, Debug)]
pub struct Label(pub u32);

#[derive(Clone, Copy, Debug)]
pub enum ConditionKind {
    Equal,
    NotEqual,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}

impl ConditionKind {
    pub fn evaluate<T: Ord>(self, lhs: T, rhs: T) -> bool {
        let order = lhs.cmp(&rhs);

        match self {
            ConditionKind::Equal => matches!(order, Ordering::Equal),
            ConditionKind::NotEqual => matches!(order, Ordering::Less | Ordering::Greater),
            ConditionKind::Greater => matches!(order, Ordering::Greater),
            ConditionKind::GreaterOrEqual => {
                matches!(order, Ordering::Greater | Ordering::Equal)
            }
            ConditionKind::Less => matches!(order, Ordering::Less),
            ConditionKind::LessOrEqual => matches!(order, Ordering::Less | Ordering::Equal),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Argument {
    Constant(Constant),
    Register(Register),
    Condition(Condition),
    Label(Label),
}

impl Argument {
    pub fn as_constant(&self) -> Option<Constant> {
        if let Self::Constant(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn as_register(&self) -> Option<Register> {
        if let Self::Register(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn as_value(&self) -> Option<Value> {
        if let Self::Constant(v) = self {
            Some(Value::Constant(*v))
        } else if let Self::Register(v) = self {
            Some(Value::Register(*v))
        } else {
            None
        }
    }

    pub fn as_condition(&self) -> Option<Condition> {
        if let Self::Condition(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn as_label(&self) -> Option<Label> {
        if let Self::Label(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}
