use std::{cmp::Ordering, io::BufRead};

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
                kind: InstructionKind::Set,
                line: 0,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('Y').unwrap(),
                    )),
                    Argument::Number(NumberSource::Constant(DigitInteger::new(10, 3).unwrap())),
                    Argument::Empty,
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Negate,
                line: 2,
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
                line: 3,
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
                line: 4,
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
                line: 5,
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
                line: 6,
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
                line: 7,
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
                line: 8,
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
                line: 9,
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
                line: 10,
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
                line: 11,
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
                line: 12,
                arguments: [
                    Argument::Comparison(Comparison {
                        ordering: Ordering::Less,
                        invert: false,
                        values: [
                            NumberSource::Register(computer::register_with_name('X').unwrap()),
                            NumberSource::Constant(DigitInteger::new(11, 3).unwrap()),
                        ],
                    }),
                    Argument::Instruction(11),
                    Argument::Empty,
                ],
            })
            .instruction(Instruction {
                kind: InstructionKind::Sleep,
                line: 13,
                arguments: [
                    Argument::Number(NumberSource::Register(
                        computer::register_with_name('X').unwrap(),
                    )),
                    Argument::Empty,
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

    println!("{:?}", computer.registers);

    let mut last_instruction = 0;

    loop {
        print!("Instruction {:?}", last_instruction);

        if let Some(instruction) = computer
            .loaded_program
            .instructions
            .get(last_instruction as usize)
        {
            print!(" ({:?})", instruction);
        }

        println!(":");

        let modified = computer.tick_partial();

        if let Some(interrupt) = computer.interrupt {
            println!("{:?}\n{:?}", interrupt, computer.registers);
            break;
        }

        if modified {
            if computer.block_time == 0 {
                println!("{:?}", computer.registers);

                last_instruction = computer.instruction;
            } else {
                println!("waiting...");
            }
        }

        if computer.tick_complete {
            println!("completed tick");

            let string = &mut String::new();

            std::io::stdin().lock().read_line(string).unwrap();

            // When not running interactively, add the missing newline
            if string.len() == 0 {
                println!();
            }
        }
    }
}
