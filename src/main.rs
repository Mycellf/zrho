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
                        values: Box::new([DigitInteger::zero(3); 100]),
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
            println!(
                "{:?}\n{}\n\nRuntime: {}",
                interrupt, computer.registers, computer.runtime,
            );
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

    SET D 2 ; COMPUTED PRIME NUMBERS
    SET X 2 ; CURRENT NUMBER

    LBL NEXT_NUMBER
    SET I 0
    ADD X 1 X

    LBL CHECK_NUMBER
    MOD X D Y

    ; JMP: 46860
    ; LJP: 67640
    ; UJP: 43315
    UJP Y = 0 NEXT_NUMBER

    ADD I 1 I
    ; JMP: 48125
    ; LJP: 43315
    ; UJP: 68850
    LJP D CHECK_NUMBER

    SET D X

    ; JMP: 43409
    ; LJP: 43315
    ; UJP: 43800
    LJP I < 99 NEXT_NUMBER
";

pub const PRIME_NUMBERS_FAST: &str = r"
    ; COMPUTE THE PRIME NUMBERS
    ; FASTER, BUT REQUIRES A THRID REGISTER

    SET D 2 ; COMPUTED PRIME NUMBERS
    SET Y 1 ; AMOUNT COMPUTED
    SET X 2 ; CURRENT NUMBER

    LBL NEXT_NUMBER
    SET I 0
    ADD X 1 X

    LBL CHECK_NUMBER
    MOD X D Z

    ; JMP: 41655
    ; LJP: 62435
    ; UJP: 38110
    UJP Z = 0 NEXT_NUMBER

    ADD I 1 I
    ; JMP: 42920
    ; LJP: 38110
    ; UJP: 63645
    LJP I < Y CHECK_NUMBER

    SET D X

    ADD Y 1 Y
    ; JMP: 38204
    ; LJP: 38110
    ; UJP: 38595
    LJP Y < 100 NEXT_NUMBER
";
