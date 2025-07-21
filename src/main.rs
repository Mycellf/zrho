use std::{env, sync::LazyLock};

use macroquad::{
    color::{Color, colors},
    input::{self, KeyCode},
    math::Vec2,
    text::{self, TextParams},
    window::Conf,
};

use crate::{
    simulation::{
        computer::{self, BlockCondition, Computer, Register, RegisterSet, RegisterValues},
        instruction,
        integer::{DigitInteger, Integer},
        program::Program,
    },
    ui::{
        header::Header,
        scroll_bar::ScrollBar,
        text_editor::TextEditor,
        text_editor_operations::TextEditorOperations,
        window::{self, Window},
    },
};

pub mod simulation;
pub mod ui;

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

    let mut window = Window::new(Vec2::new(50.0, 50.0), 400.0, true);

    window.theme.accent_color = ui::colors::BLUE;

    window.push_element(Header {
        title: "Kolakoski Sequence".to_owned(),
    });

    window.push_element(ScrollBar::new(TextEditor::new(
        TextEditorOperations::new(KOLAKOSKI_SEQUENCE_LONG.to_owned()),
        35.0,
        2.5,
    )));

    windows.push(window);

    let mut window = Window::new(Vec2::new(500.0, 50.0), 400.0, false);

    window.theme.accent_color = ui::colors::FUSCHIA;

    window.push_element(Header {
        title: "Prime Numbers".to_owned(),
    });

    window.push_element(ScrollBar::new(TextEditor::new(
        TextEditorOperations::new(PRIME_NUMBERS.to_owned()),
        21.0,
        2.5,
    )));

    windows.push(window);

    let mut window = Window::new(Vec2::new(950.0, 50.0), 400.0, false);

    window.push_element(Header {
        title: "Fibonacci Sequence".to_owned(),
    });

    window.theme.accent_color = ui::colors::RED;

    window.push_element(ScrollBar::new(TextEditor::new(
        TextEditorOperations::new(FIBONACCI_SEQUENCE.to_owned()),
        17.0,
        2.5,
    )));

    windows.push(window);

    // HACK: Macroquad renders the first bit of text drawn as black boxes,
    // so draw everything twice on the first frame.
    for _ in 0..2 {
        for window in &mut windows {
            for element in &mut window.elements {
                element.force_update();
            }

            window.update_all = true;
            window.update_texture();
        }
    }

    loop {
        if input::is_key_pressed(KeyCode::F11) {
            fullscreen ^= true;
            macroquad::window::set_fullscreen(fullscreen);
        }

        ui::update_key_repeats();

        let mut found_hovered_window = false;

        for i in 0..windows.len() {
            let clicked = windows[i].update(&mut found_hovered_window);

            if clicked && i > 0 {
                let front_window = windows.first_mut().unwrap();
                front_window.is_focused = false;
                front_window.update_focus = true;

                let mut window = windows.remove(i);
                window.is_focused = true;
                window.update_focus = true;

                windows.insert(0, window);
            }
        }

        ui::clear_chars_typed();

        macroquad::window::clear_background(Color::from_hex(0x08080b));

        for window in windows.iter_mut().rev() {
            window.draw();
        }

        if *DRAW_FPS {
            // BUG: The windows build (tested with bottles) has all the text replaced with black
            // boxes if this is turned on, but only after the first frame is run.
            text::draw_text_ex(
                &macroquad::time::get_fps().to_string(),
                10.0,
                50.0,
                TextParams {
                    color: colors::WHITE,
                    ..window::text_params_with_height(40.0)
                },
            );
        }

        macroquad::window::next_frame().await;
    }
}

static DRAW_FPS: LazyLock<bool> =
    LazyLock::new(|| env::args().skip(1).any(|argument| argument == "--draw-fps"));

pub fn run_test_computer() {
    let mut computer = default_computer(true);

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

    for (i, (computed, actual)) in register.all_values().iter().zip(sequence).enumerate() {
        assert_eq!(
            computed.get(),
            actual,
            "Element {} is incorrect",
            i as Integer + register.offset()
        );
    }

    println!("Verified Kolakoski Sequence stored in H");
}

#[must_use]
pub fn default_computer(h_register: bool) -> Computer {
    const DIGITS: u8 = 3;

    let registers = RegisterSet::new_empty()
        .with_register(
            'D',
            Register {
                values: RegisterValues::Vector {
                    values: Box::new([DigitInteger::new(0, DIGITS).unwrap(); 100]),
                    index: 0,
                    offset: 0,
                },
                indexed_by: Some(computer::register_with_name('I').unwrap()),
                read_time: 1,
                write_time: 1,
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
        );

    let registers = if h_register {
        registers
            .with_register(
                'H',
                Register {
                    values: RegisterValues::Vector {
                        values: vec![DigitInteger::new(0, DIGITS).unwrap(); 10000]
                            .into_boxed_slice(),
                        index: 0,
                        offset: 0,
                    },
                    block_condition: Some(BlockCondition::IndexChange {
                        minimum_change: 2,
                        block_time: 16,
                    }),
                    indexed_by: Some(computer::register_with_name('M').unwrap()),
                    read_time: 2,
                    write_time: 4,
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
    } else {
        registers
    };

    Computer::new(DIGITS, registers, instruction::DEFAULT_INSTRUCTIONS)
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
