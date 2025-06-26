use crate::{
    computer::{Computer, Program, Register, RegisterSet, RegisterValues},
    instruction::{Argument, Instruction, InstructionKind, NumberSource},
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
                kind: InstructionKind::Add,
                line: 2,
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
                line: 3,
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
                line: 4,
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
                line: 5,
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
                line: 6,
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
                line: 7,
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
            }),
        RegisterSet::new()
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
                    read_time: 0,
                    write_time: 0,
                },
            ),
    );

    loop {
        if computer.block_time == 0 {
            println!("{:?}\n", computer.registers);
        } else {
            println!("waiting...");
        }

        computer.tick();

        if let Some(interrupt) = computer.interrupt {
            println!("{:?}\n\n{:?}", interrupt, computer.registers);
            break;
        }
    }
}
