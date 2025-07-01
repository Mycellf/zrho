use crate::simulation::{
    computer::{self, Computer, Register, RegisterMap, RegisterSet, RegisterValues},
    integer::DigitInteger,
    program::Program,
};

pub mod simulation;

fn main() {
    const DIGITS: u8 = 3;

    let program = match Program::assemble_from(
        "Test Program".to_owned(),
        PROGRAM,
        RegisterMap::from_element(false)
            .with_value('D', true)
            .with_value('I', true)
            .with_value('X', true)
            .with_value('Y', true),
    ) {
        Ok(program) => program,
        Err(errors) => {
            for error in errors {
                println!("{error}");
            }
            return;
        }
    };

    let computer = Computer::new(
        program,
        RegisterSet::new_empty()
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
            ),
    );

    simulation::interactively_run(computer);
}

const PROGRAM: &str = PRIME_NUMBERS;

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
