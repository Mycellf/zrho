use crate::simulation::{
    computer::{self, Computer, Register, RegisterSet, RegisterValues},
    instruction::CustomInstructionProperties,
    integer::DigitInteger,
    program::Program,
};

pub mod simulation;
pub mod text_editor;

fn main() {
    const DIGITS: u8 = 3;

    let computer = Computer::new(
        DIGITS,
        RegisterSet::new_empty()
            .with_register(
                'D',
                Register {
                    values: RegisterValues::Vector {
                        values: Box::new([DigitInteger::new(0, DIGITS).unwrap(); 100]),
                        index: 0,
                    },
                    indexes_array: None,
                    read_time: 1,
                    write_time: 1,
                },
            )
            .with_register(
                'I',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::new(0, DIGITS).unwrap()),
                    indexes_array: Some(computer::register_with_name('D').unwrap()),
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                'X',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::new(0, DIGITS).unwrap()),
                    indexes_array: None,
                    read_time: 0,
                    write_time: 0,
                },
            )
            .with_register(
                'Y',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::new(0, DIGITS).unwrap()),
                    indexes_array: None,
                    read_time: 0,
                    write_time: 0,
                },
            ),
        CustomInstructionProperties::default(),
    );

    let program = match Program::assemble_from("Test Program".to_owned(), PROGRAM, &computer) {
        Ok(program) => program,
        Err(errors) => {
            for error in errors {
                println!("{error}");
            }
            return;
        }
    };

    simulation::interactively_run(computer, program);
}

const PROGRAM: &str = TIME_ASSERTION;

pub const TIME_ASSERTION: &str = r"
    ; ENSURE THAT THE DURATION OF PROGRAM EXECUTION
    ; IS EXACTLY AS EXPECTED

    ; PREVIOUS CODE
    SET I 99
    SLP 9999
    SLP 9999
    SLP 3124

    ; SOMETHING NEFARIOUS EXTERNALLY INSERTED
    SET I 10

    ; BECAUSE OF THE SECOND DIGIT CHECK, IT
    ; ONLY OVERFLOWS AFTER 10^8 TICKS
    SET X 0
    LBL WAIT
    SLP 9998
    ADD X 1 X
    JMP X < 9999 WAIT
    SLP 9998

    CLK X
    UJP X ≠ 3123 ALTERED
    CLK X 4
    LJP X = 2 CONTINUE
    LBL ALTERED
    END

    LBL CONTINUE
    SET D 9999
";

pub const RANDOM_TESTS: &str = r"
    ; NOODLE AROUND

    CLK X

    TRY D
    TRW D

    CLK Y

    JMP 0 END
    UJP 0 END
    LJP 1 NEXT
    LBL NEXT

    ADD 3 2 X
    NEG X
    SET Y 10

    NEG Y
    SUB Y 1 Y
    NEG X
    SUB Y X Y

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
    SET X 3 ; CURRENT NUMBER
    JMP CHECK_NUMBER

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
    SET X 3 ; CURRENT NUMBER
    JMP CHECK_NUMBER

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
