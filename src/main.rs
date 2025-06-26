use std::cmp::Ordering;

use crate::{
    computer::{Computer, Program, Register, RegisterSet, RegisterValues},
    instruction::{Argument, Comparison, Instruction, InstructionKind, NumberSource},
    integer::DigitInteger,
};

pub mod computer;
pub mod instruction;
pub mod integer;

fn main() {
    let mut computer = Computer::new(
        Program::new_empty("Test Program".to_owned())
            .instruction(Instruction {
                kind: InstructionKind::Add,
                line: 0,
                arguments: [
                    Argument::Number(NumberSource::Constant(DigitInteger::new(3, 3).unwrap())),
                    Argument::Number(NumberSource::Constant(DigitInteger::new(2, 3).unwrap())),
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('X').unwrap(),
                    )),
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Negate,
                line: 1,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('X').unwrap(),
                    )),
                    Argument::Empty,
                    Argument::Empty,
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Negate,
                line: 2,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('Y').unwrap(),
                    )),
                    Argument::Empty,
                    Argument::Empty,
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Add,
                line: 3,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('X').unwrap(),
                    )),
                    Argument::Number(NumberSource::Constant(DigitInteger::new(1, 3).unwrap())),
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('D').unwrap(),
                    )),
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Set,
                line: 4,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('I').unwrap(),
                    )),
                    Argument::Number(NumberSource::Constant(DigitInteger::new(1, 3).unwrap())),
                    Argument::Empty,
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Set,
                line: 5,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('D').unwrap(),
                    )),
                    Argument::Number(NumberSource::Constant(DigitInteger::new(1, 3).unwrap())),
                    Argument::Empty,
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Multiply,
                line: 6,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('I').unwrap(),
                    )),
                    Argument::Number(NumberSource::Constant(DigitInteger::new(2, 3).unwrap())),
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('I').unwrap(),
                    )),
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Divide,
                line: 7,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('X').unwrap(),
                    )),
                    Argument::Number(NumberSource::Constant(DigitInteger::new(2, 3).unwrap())),
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('Y').unwrap(),
                    )),
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Modulus,
                line: 8,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('X').unwrap(),
                    )),
                    Argument::Number(NumberSource::Constant(DigitInteger::new(2, 3).unwrap())),
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('I').unwrap(),
                    )),
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Divide,
                line: 9,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('X').unwrap(),
                    )),
                    Argument::Number(NumberSource::Constant(DigitInteger::new(2, 3).unwrap())),
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('Y').unwrap(),
                    )),
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Add,
                line: 10,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('X').unwrap(),
                    )),
                    Argument::Number(NumberSource::Constant(DigitInteger::new(1, 3).unwrap())),
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('X').unwrap(),
                    )),
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::JumpCondLikely,
                line: 11,
                arguments: [
                    Argument::Comparison(Comparison {
                        ordering: Ordering::Less,
                        invert: false,
                        values: [
                            NumberSource::Register(computer::register_with_name('X').unwrap()),
                            NumberSource::Constant(DigitInteger::new(10, 3).unwrap()),
                        ],
                    }),
                    Argument::Instruction(10),
                    Argument::Empty,
                ],
            }),
        RegisterSet::new_empty()
            .with_register(
                computer::register_with_name('X').unwrap(),
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(3)),
                    indexes_array: None,
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                computer::register_with_name('Y').unwrap(),
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(3)),
                    indexes_array: None,
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                computer::register_with_name('I').unwrap(),
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(3)),
                    indexes_array: Some(computer::register_with_name('D').unwrap()),
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                computer::register_with_name('D').unwrap(),
                Register {
                    values: RegisterValues::Vector {
                        values: Box::new([DigitInteger::zero(3); 2]),
                        index: 0,
                    },
                    indexes_array: None,
                    read_time: 1,
                    write_time: 1,
                },
            ),
    );

    println!("{:?}\n", computer.registers);

    loop {
        println!("{:?}", computer.instruction);

        computer.tick_partial();

        if let Some(interrupt) = computer.interrupt {
            println!("{:?}\n\n{:?}", interrupt, computer.registers);
            break;
        }

        if computer.block_time == 0 {
            println!("{:?}\n", computer.registers);
        } else {
            println!("waiting...");
        }

        if computer.tick_complete {
            println!("completed tick\n");
        }
    }
}
