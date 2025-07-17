use macroquad::{
    color::{Color, colors},
    math::Vec2,
    shapes,
    text::{self, TextParams},
};

use crate::{
    interface::{
        FONT,
        editor_window::{EditorWindow, exp_decay_cutoff},
    },
    simulation::computer::{self, Register, RegisterValues},
};

#[derive(Clone, Copy, Debug)]
pub struct RegisterVisualisationLayout {}

#[derive(Clone, Copy, Debug)]
pub struct RegisterVisualisation {
    pub register: u32,
    pub value_visualisation: ValueVisualisation,
}

impl RegisterVisualisation {
    pub const NAME_WIDTH: f32 = EditorWindow::TEXT_WIDTH * 8.0;
    pub const HEIGHT: f32 = EditorWindow::TEXT_SIZE * 2.5;

    pub fn new(index: u32, register: &Register) -> Self {
        Self {
            register: index,
            value_visualisation: match register.values {
                RegisterValues::Scalar(..) => ValueVisualisation::Scalar,
                RegisterValues::Vector { .. } => ValueVisualisation::Vector {
                    index: 0,
                    scroll: 0.0,
                    target_scroll: 0.0,
                },
            },
        }
    }

    pub fn update(&mut self, register: &Register) {
        self.value_visualisation.update(register);
    }

    pub fn draw_at(&self, location: Vec2, register: &Register, title_color: Color) {
        shapes::draw_rectangle(
            location.x,
            location.y,
            Self::NAME_WIDTH,
            EditorWindow::TEXT_SIZE,
            title_color,
        );

        let name = computer::name_of_register(self.register).unwrap();

        Self::draw_centered_text(
            &name.to_string(),
            location,
            Self::NAME_WIDTH,
            EditorWindow::EDITOR_BACKGROUND_COLOR,
        );

        let (value, is_error) = match register.value() {
            Ok(value) => (value.to_string(), false),
            Err(error) => (
                match error {
                    computer::RegisterAccessError::IndexTooBig { maximum, .. } => {
                        format!(
                            "{}>{maximum}",
                            computer::name_of_register(register.indexed_by.unwrap()).unwrap(),
                        )
                    }
                    computer::RegisterAccessError::IndexTooSmall { minimum, .. } => {
                        format!(
                            "{}<{minimum}",
                            computer::name_of_register(register.indexed_by.unwrap()).unwrap(),
                        )
                    }
                    _ => unreachable!(),
                },
                true,
            ),
        };

        Self::draw_centered_text(
            &value,
            location + Vec2::new(0.0, EditorWindow::TEXT_SIZE),
            Self::NAME_WIDTH,
            if is_error {
                Color::from_hex(0xff0000)
            } else {
                colors::WHITE
            },
        );

        match self.value_visualisation {
            ValueVisualisation::Scalar => {
                assert!(matches!(register.values, RegisterValues::Scalar(..)));
            }
            ValueVisualisation::Vector {
                index,
                scroll,
                target_scroll,
            } => {
                let RegisterValues::Vector {
                    values,
                    index: register_index,
                    offset,
                } = &register.values
                else {
                    panic!();
                };

                // TODO:
            }
        }
    }

    fn draw_centered_text(text: &str, location: Vec2, width: f32, color: Color) {
        let TextParams {
            font_size,
            font_scale,
            ..
        } = EditorWindow::text_params_with_size(EditorWindow::TEXT_SIZE);

        let center = text::get_text_center(text, Some(&FONT), font_size, font_scale, 0.0);

        text::draw_text_ex(
            text,
            location.x + width / 2.0 - center.x,
            location.y + EditorWindow::TEXT_SIZE * 0.875,
            TextParams {
                color,
                ..EditorWindow::text_params_with_size(EditorWindow::TEXT_SIZE)
            },
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ValueVisualisation {
    Scalar,
    Vector {
        index: usize,
        scroll: f32,
        target_scroll: f32,
    },
}

impl ValueVisualisation {
    pub fn update(&mut self, register: &Register) {
        match self {
            ValueVisualisation::Scalar => {
                assert!(matches!(register.values, RegisterValues::Scalar(..)));
            }
            ValueVisualisation::Vector {
                index,
                scroll,
                target_scroll,
            } => {
                let RegisterValues::Vector {
                    values: _,
                    index: register_index,
                    offset,
                } = &register.values
                else {
                    panic!();
                };

                let new_index = *register_index as usize + *offset as usize;

                if *index != new_index {
                    *index = new_index;

                    *target_scroll = *register_index as f32 + *offset as f32;
                }

                *scroll = exp_decay_cutoff(
                    *scroll,
                    *target_scroll,
                    25.0,
                    macroquad::time::get_frame_time(),
                    0.01,
                )
                .0;
            }
        }
    }
}
