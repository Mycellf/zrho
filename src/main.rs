use std::io::BufRead;

use crate::{
    computer::{Computer, Register, RegisterMap, RegisterSet, RegisterValues},
    integer::DigitInteger,
    program::Program,
};

pub mod argument;
pub mod computer;
pub mod instruction;
pub mod integer;
pub mod program;

fn main() {
    let mut computer = Computer::new(
        Program::assemble_from(
            "Test Program".to_owned(),
            r"
                ; NOODLE AROUND

                ADD 3 2 X
                SET Y 10

                NEG X
                NEG Y

                ADD X 1 D
                SET I 1
                SET D 1
                MUL I 2 I

                DIV X 2 Y
                MOD X 2 I
                DIV X 2 Y

                LBL LOOP
                ADD X 1 X
                LJP X < 11 LOOP

                SLP X

                SLP 0
                SLP 0
                SLP 0

                ; COMPUTE THE FIBONACCI SEQUENCE

                SET I 0
                SET X 1
                SET Y 0

                LBL FIBONACCI

                ADD X Y X
                SET D X
                ADD I 1 I

                ADD X Y Y
                SET D Y
                ADD I 1 I

                LJP I < 20 FIBONACCI
            ",
            RegisterMap::from_element(false)
                .with_value('D', true)
                .with_value('I', true)
                .with_value('X', true)
                .with_value('Y', true),
        )
        .unwrap(),
        RegisterSet::new_empty()
            .with_register(
                'X',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(3)),
                    indexes_array: None,
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                'Y',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(3)),
                    indexes_array: None,
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                'I',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(3)),
                    indexes_array: Some(computer::register_with_name('D').unwrap()),
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                'D',
                Register {
                    values: RegisterValues::Vector {
                        values: Box::new([DigitInteger::zero(3); 25]),
                        index: 0,
                    },
                    indexes_array: None,
                    read_time: 1,
                    write_time: 1,
                },
            ),
    );

    println!("{}", computer.registers);

    let mut last_instruction = 0;

    loop {
        print!("Instruction {:?}", last_instruction);

        if let Some(instruction) = computer
            .loaded_program
            .instructions
            .get(last_instruction as usize)
        {
            print!(" ({instruction})");
        }

        println!(":");

        let modified = computer.tick_partial();

        if let Some(interrupt) = computer.interrupt {
            println!("{:?}\n{}", interrupt, computer.registers);
            break;
        }

        if modified {
            if computer.block_time == 0 {
                println!("{}", computer.registers);

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
            if string.is_empty() {
                println!();
            }
        }
    }
}
