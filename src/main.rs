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
    const DIGITS: u8 = 3;

    let mut computer = Computer::new(
        Program::assemble_from(
            "Test Program".to_owned(),
            PROGRAM,
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
                    values: RegisterValues::Scalar(DigitInteger::zero(DIGITS)),
                    indexes_array: None,
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                'Y',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(DIGITS)),
                    indexes_array: None,
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                'I',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::zero(DIGITS)),
                    indexes_array: Some(computer::register_with_name('D').unwrap()),
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                'D',
                Register {
                    values: RegisterValues::Vector {
                        values: Box::new([DigitInteger::zero(DIGITS); 100]),
                        index: 0,
                    },
                    indexes_array: None,
                    read_time: 1,
                    write_time: 1,
                },
            ),
    );

    let mut skip_ticks = 0;

    loop {
        let instruction = computer.instruction;

        let modified = computer.tick_partial();

        if let Some(interrupt) = computer.interrupt {
            println!(
                "\n{:?}\n{}\n\nRuntime: {}",
                interrupt, computer.registers, computer.runtime,
            );
            break;
        }

        if skip_ticks == 0 && computer.block_time == 0 {
            print!("Instruction {:?}", instruction);

            if let Some(instruction) = computer
                .loaded_program
                .instructions
                .get(instruction as usize)
            {
                print!(" ({instruction})");
            }

            println!(":");

            if modified {
                println!("{}", computer.registers);
            }
        }

        if computer.tick_complete {
            if skip_ticks == 0 {
                if computer.block_time == 0 {
                    println!("completed tick");
                } else {
                    println!(
                        "waiting {} tick{}...",
                        computer.block_time,
                        if computer.block_time == 1 { "" } else { "s" },
                    );
                }
            } else {
                skip_ticks -= 1;
            }
        }

        if skip_ticks == 0 && computer.block_time == 0 {
            let string = &mut String::new();

            std::io::stdin().lock().read_line(string).unwrap();

            if string.is_empty() {
                // When not running interactively, add the missing newline
                println!();
            } else if ["e", "end"].contains(&string.trim()) {
                skip_ticks = u64::MAX;
            } else if let Ok(input) = string.trim().parse::<u64>() {
                skip_ticks = input;
            }
        }
    }
}

const PROGRAM: &str = PRIME_NUMBERS;

pub const RANDOM_TESTS: &str = r"
    ; NOODLE AROUND

    ADD 3 2 X
    SET Y 10

    NEG X
    NEG Y

    ADD X 1 D
    ADD D D D
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

    JMP END
    SLP 9999
    LBL END
";

pub const FIBONACCI_SEQUENCE: &str = r"
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
";

pub const PRIME_NUMBERS: &str = r"
    ; COMPUTE THE PRIME NUMBERS

    SET D 2 ; PRIME NUMBERS
    SET X 1 ; CURRENT NUMBER

    LBL NEXT_NUMBER
    SET I 0
    ADD X 2 X

    LBL CHECK_NUMBER
    MOD X D Y

    UJP Y = 0 NEXT_NUMBER

    ADD I 1 I
    LJP D CHECK_NUMBER

    SET D X

    LJP I < 99 NEXT_NUMBER
";

pub const PRIME_NUMBERS_FAST: &str = r"
    ; COMPUTE THE PRIME NUMBERS
    ; FASTER, BUT REQUIRES A THRID REGISTER

    SET D 2 ; PRIME NUMBERS
    SET Y 1 ; AMOUNT COMPUTED
    SET X 1 ; CURRENT NUMBER

    LBL NEXT_NUMBER
    SET I 0
    ADD X 2 X

    LBL CHECK_NUMBER
    MOD X D Z

    UJP Z = 0 NEXT_NUMBER

    ADD I 1 I
    LJP I < Y CHECK_NUMBER

    SET D X

    ADD Y 1 Y
    LJP Y < 100 NEXT_NUMBER
";
