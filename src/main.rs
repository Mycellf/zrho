use macroquad::{
    input::{self, KeyCode},
    math::Vec2,
    window::{self, Conf},
};

use crate::{
    interface::{text_editor::TextEditor, window::EditorWindow},
    simulation::{
        computer::{self, BlockCondition, Computer, Register, RegisterSet, RegisterValues},
        instruction,
        integer::{DigitInteger, Integer},
        program::Program,
    },
};

pub mod interface;
pub mod simulation;

const START_IN_FULLSCREEN: bool = false;

fn config() -> Conf {
    Conf {
        window_title: "zρ".to_owned(),
        fullscreen: START_IN_FULLSCREEN,
        ..Default::default()
    }
}

#[macroquad::main(config)]
async fn main() {
    let mut fullscreen = START_IN_FULLSCREEN;

    let text_editor = TextEditor::new(KOLAKOSKI_SEQUENCE_LONG.to_owned());

    let mut window = EditorWindow::new(
        Vec2::new(10.0, 10.0),
        Vec2::new(100.0, 200.0),
        "KOLAKOSKI SEQUENCE".to_owned(),
        text_editor,
        &default_computer(),
    );

    loop {
        if input::is_key_pressed(KeyCode::F11) {
            fullscreen ^= true;
            window::set_fullscreen(fullscreen);
        }

        window.update();

        window.draw();

        window::next_frame().await;
    }
}

pub fn run_test_computer() {
    let mut computer = default_computer();

    let program = match Program::assemble_from("Test Program".to_owned(), PROGRAM, &computer) {
        Ok(program) => program,
        Err(errors) => {
            for error in errors {
                println!("{error}");
            }
            return;
        }
    };

    println!(
        "Program length: {} instructions",
        program.instructions.len(),
    );

    simulation::interactively_run(&mut computer, &program);

    if !std::ptr::eq(PROGRAM, KOLAKOSKI_SEQUENCE_LONG) {
        return;
    }

    let length = computer
        .registers
        .get(computer::register_with_name('H').unwrap())
        .unwrap()
        .all_values()
        .len();

    let mut sequence = vec![1, 2, 2];
    let mut i = 2;

    while sequence.len() < length {
        for _ in 0..sequence[i] {
            sequence.push((i % 2 + 1) as i32);
        }

        i += 1;
    }

    // There may be an excess element
    while sequence.len() > length {
        sequence.pop();
    }

    let register = computer
        .registers
        .get(computer::register_with_name('H').unwrap())
        .unwrap();

    for (i, (computed, actual)) in register.all_values().into_iter().zip(sequence).enumerate() {
        assert_eq!(
            computed.get(),
            actual,
            "Element {} is incorrect",
            i as Integer + register.offset()
        );
    }

    println!("Verified Kolakoski Sequence stored in H");
}

pub fn default_computer() -> Computer {
    const DIGITS: u8 = 3;

    Computer::new(
        DIGITS,
        RegisterSet::new_empty()
            .with_register(
                'D',
                Register {
                    values: RegisterValues::Vector {
                        values: Box::new([DigitInteger::new(0, DIGITS).unwrap(); 100]),
                        index: 0,
                        offset: 0,
                    },
                    read_time: 1,
                    write_time: 1,
                    ..Register::DEFAULT
                },
            )
            .with_register(
                'H',
                Register {
                    values: RegisterValues::Vector {
                        values: Box::new([DigitInteger::new(0, DIGITS).unwrap(); 10000]),
                        index: 0,
                        offset: 0,
                    },
                    block_condition: Some(BlockCondition::IndexChange {
                        minimum_change: 2,
                        block_time: 16,
                    }),
                    read_time: 2,
                    write_time: 4,
                    ..Register::DEFAULT
                },
            )
            .with_register(
                'I',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::new(0, DIGITS).unwrap()),
                    indexes_array: Some(computer::register_with_name('D').unwrap()),
                    ..Register::DEFAULT
                },
            )
            .with_register(
                'M',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::new(0, DIGITS).unwrap()),
                    indexes_array: Some(computer::register_with_name('H').unwrap()),
                    ..Register::DEFAULT
                },
            )
            .with_register(
                'X',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::new(0, DIGITS).unwrap()),
                    ..Register::DEFAULT
                },
            )
            .with_register(
                'Y',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::new(0, DIGITS).unwrap()),
                    ..Register::DEFAULT
                },
            )
            .with_register(
                'Z',
                Register {
                    values: RegisterValues::Scalar(DigitInteger::new(0, DIGITS).unwrap()),
                    ..Register::DEFAULT
                },
            ),
        instruction::DEFAULT_INSTRUCTIONS,
    )
}

const PROGRAM: &str = KOLAKOSKI_SEQUENCE_LONG;

pub const KOLAKOSKI_SEQUENCE_LONG: &str = r"
    ; COMPUTES AND STORES THE FIRST 10000 ELEMENTS OF THE
    ; KOLAKOSKI SEQUENCE TO H

    ; USES D AS A BUFFER TO MINIMIZE TIME SPENT WITH H BLOCKING

    ; MAXIMUM BUFFER SIZE STATISTICS (COLLECTED WITH THE SIZE OF D INCREASED)
    ; SIZE  RUNTIME
    ; 10    193119
    ; 50    158243
    ; 100   154233  (USED)
    ; 150   152869
    ; 200   152225
    ; 250   151730
    ; ...
    ; 500   150982
    ; 550   150734
    ; 600   150795
    ; 650   150776
    ; 700   150672
    ; 750   150500
    ; 800   150833  (INCREASES)
    ; 850   150559
    ; 900   150220  (BEST)
    ; 950   150486  (INCREASES)
    ; 1000  150722  (INCREASES)
    ; 2000  149524
    ; 3000  150297  (INCREASES)
    ; 3333  150466  (INCREASES)
    ; 3334  150466
    ; 3400  150466
    ; 3500  INCORRECT RESULT AT M = 9794 (RUNS OUT OF ELEMENTS IN H DURING FINAL PASS)
    ; 4000  INCORRECT RESULT AT M = 9794

    ; STORE THE FIRST FEW ELEMENTS
    SET H 1
    SET M 1
    SET H 2
    SET M 2
    SET H 2

    ; M IS 2 NOW, WHICH IS THE FIRST INDEX TO READ

    SET Y 2 ; INDEX OF THE LAST WRITTEN ELEMENT

    SET Z 1 ; NEXT ELEMENT

    ; FILL D AS MUCH AS POSSIBLE
    LBL GENERATE
    SET X H
    UJP X = 0 BREAK

    SET D Z
    JMP X = 1 CONTINUE
    ADD I 1 I
    SET D Z

    LBL CONTINUE
    SUB 3 Z Z ; TOGGLE Z BETWEEN 1 AND 2
    ADD I 1 I
    ADD M 1 M
    LJP I < 99 GENERATE ; CONTINUE IF THE BUFFER IS AT ITS SIZE LIMIT OR MIGHT
                        ; EXCEED IT NEXT ITERATION
    
    LBL BREAK

    ; COPY NEW VALUES FROM D TO H
    SET X M
    SET M Y

    ADD Y I Y

    LBL COPY_TO_H
    CLK I 9999

    LBL COPY_TO_H_LOOP
    ADD M 1 M
    SET H D
    UJP M = 9999 END
    ADD I 1 I
    LJP M < Y COPY_TO_H_LOOP

    CLK I 9999
    SET M X

    LJP Y ≤ 9900 GENERATE
    SUB 9999 Y Y

    ; GENERATE A BUFFER WITH ONLY THE AMOUNT NEEDED TO FILL H
    LBL GENERATE_END
    SET D Z
    JMP H = 1 CONTINUE_END
    ADD I 1 I
    SET D Z

    LBL CONTINUE_END
    SUB 3 Z Z ; TOGGLE Z BETWEEN 1 AND 2
    ADD I 1 I
    ADD M 1 M
    LJP I < Y GENERATE_END ; CONTINUE IF THE BUFFER CAN FILL THE REST OF H

    SUB 9999 Y M
    SET Y 9999
    JMP COPY_TO_H

    LBL END
";

pub const KOLAKOSKI_SEQUENCE: &str = r"
    ; COMPUTES AND STORES THE FIRST 100 ELEMENTS OF THE
    ; KOLAKOSKI SEQUENCE TO D

    ; X IS THE INDEX TO READ FROM
    ; Y IS THE INDEX TO WRITE TO

    LBL LOOP
    SET I X
    SET Z D

    LJP Z ADD_FROM_ELEMENT
    ADD Y 1 Z

    LBL ADD_FROM_ELEMENT
    SET I Y
    UJP I ≥ 100 END
    MOD X 2 Y

    ADD Y 1 D
    JMP Z = 1 CONTINUE
    ADD I 1 I
    ADD Y 1 D

    LBL CONTINUE
    ADD I 1 Y
    ADD X 1 X
    LJP Y < 100 LOOP

    LBL END
";

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

    SET X 0
    LBL WAIT
    SLP 9998
    ADD X 1 X
    JMP X < 9999 WAIT
    SLP 9998

    ; BECAUSE OF THE SECOND CLOCK CHECK, IT OVERFLOWS
    ; AFTER 10⁸ TICKS IN STEAD OF 10⁴ TICKS
    CLK X
    UJP X ≠ 3123 ALTERED
    CLK X 4
    LJP X = 2 OK
    LBL ALTERED
    END

    LBL OK
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
    CLK I 9999
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
    CLK I 9999
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
