use macroquad::{
    input::{self, KeyCode},
    math::Vec2,
    window::{self, Conf},
};

use crate::{
    interface::{
        text_editor::TextEditor,
        window::{EditorWindow, WindowFocus},
    },
    simulation::{
        computer::{self, BlockCondition, Computer, Register, RegisterSet, RegisterValues},
        instruction,
        integer::{DigitInteger, Integer},
        program::Program,
    },
};

pub mod interface;
pub mod simulation;

const START_IN_FULLSCREEN: bool = true;

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

    let mut windows = Vec::new();

    windows.push(EditorWindow::new(
        Vec2::new(50.0, 50.0),
        Vec2::new(400.0, 700.0),
        100.0,
        "KOLAKOSKI SEQUENCE".to_owned(),
        EditorWindow::BLUE,
        TextEditor::new(KOLAKOSKI_SEQUENCE_LONG.to_owned()),
        default_computer(),
    ));

    windows.push(EditorWindow::new(
        Vec2::new(550.0, 50.0),
        Vec2::new(400.0, 400.0),
        0.0,
        "PRIME NUMBERS".to_owned(),
        EditorWindow::ORANGE,
        TextEditor::new(PRIME_NUMBERS_FAST.to_owned()),
        default_computer(),
    ));

    windows.push(EditorWindow::new(
        Vec2::new(1050.0, 50.0),
        Vec2::new(400.0, 300.0),
        0.0,
        "FIBONACCI SEQUENCE".to_owned(),
        EditorWindow::RED,
        TextEditor::new(FIBONACCI_SEQUENCE.to_owned()),
        default_computer(),
    ));

    windows[0].is_focused = true;

    loop {
        if input::is_key_pressed(KeyCode::F11) {
            fullscreen ^= true;
            window::set_fullscreen(fullscreen);
        }

        let mut focus = WindowFocus::default();

        let mut i = 0;
        while i < windows.len() {
            let window = &mut windows[i];

            if focus.grab.is_none() {
                if window.should_hold_mouse_focus() {
                    focus.grab = Some(i);
                    focus.mouse = None;
                }

                if focus.mouse.is_none()
                    && window.is_point_within_bounds(input::mouse_position().into())
                {
                    focus.mouse = Some(i);
                }
            }

            let is_clicked = window.update(focus, i) && focus.grab.is_none();

            if !is_clicked {
                i += 1;
            } else {
                focus.grab = Some(i);

                if i > 0 {
                    let front_window = &mut windows[0];
                    front_window.is_focused = false;
                    front_window.contents_updated = true;

                    let mut window = windows.remove(i);
                    window.is_focused = true;
                    window.contents_updated = true;
                    windows.insert(0, window);
                }
            }
        }

        window::clear_background(EditorWindow::BACKGROUND_COLOR);

        for window in windows.iter_mut().rev() {
            window.draw();
        }

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

pub const KOLAKOSKI_SEQUENCE_LONG: &str =
    include_str!("../assets/examples/kolakoski_sequence_long.zρ");

pub const KOLAKOSKI_SEQUENCE: &str = include_str!("../assets/examples/kolakoski_sequence.zρ");

pub const TIME_ASSERTION: &str = include_str!("../assets/examples/time_assertion.zρ");

pub const RANDOM_TESTS: &str = include_str!("../assets/examples/random_tests.zρ");

pub const FIBONACCI_SEQUENCE: &str = include_str!("../assets/examples/fibonacci_sequence.zρ");

pub const PRIME_NUMBERS: &str = include_str!("../assets/examples/prime_numbers.zρ");

pub const PRIME_NUMBERS_FAST: &str = include_str!("../assets/examples/prime_numbers_fast.zρ");
