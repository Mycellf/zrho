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
    simulation::computer::{self, Computer, Register, RegisterValues},
};

#[derive(Clone, Debug)]
pub struct RegisterVisualisationLayout {
    pub visualisations: Vec<RegisterVisualisation>,
}

impl RegisterVisualisationLayout {
    pub fn new(computer: &Computer) -> Self {
        let visualisations = (computer.registers.registers)
            .iter()
            .enumerate()
            .filter_map(|(i, register)| register.as_ref().map(|register| (i, register)))
            .map(|(i, register)| RegisterVisualisation::new(i.try_into().unwrap(), register))
            .collect();

        Self { visualisations }
    }

    pub fn update(&mut self, computer: &Computer) {
        for register_visualisation in &mut self.visualisations {
            register_visualisation.update(
                computer
                    .registers
                    .get(register_visualisation.register)
                    .unwrap(),
            );
        }
    }

    pub fn draw_at(&self, location: Vec2, computer: &Computer, color: Color) {
        let mut column_height = [0.0; 4];
        let column_width = RegisterVisualisation::NAME_WIDTH + EditorWindow::TEXT_WIDTH;

        for register_visualisation in &self.visualisations {
            let column = computer::column_of_register(register_visualisation.register);

            register_visualisation.draw_at(
                location + Vec2::new(column as f32 * column_width, column_height[column]),
                computer
                    .registers
                    .get(register_visualisation.register)
                    .unwrap(),
                color,
            );

            column_height[column] += RegisterVisualisation::HEIGHT;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RegisterVisualisation {
    pub register: u32,
    pub value_visualisation: ValueVisualisation,
}

impl RegisterVisualisation {
    pub const NAME_WIDTH: f32 = EditorWindow::TEXT_WIDTH * 5.0;
    pub const HEIGHT: f32 = EditorWindow::TEXT_SIZE * 2.5;

    #[must_use]
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

        // match self.value_visualisation {
        //     ValueVisualisation::Scalar => {
        //         assert!(matches!(register.values, RegisterValues::Scalar(..)));
        //     }
        //     ValueVisualisation::Vector {
        //         index,
        //         scroll,
        //         target_scroll,
        //     } => {
        //         let RegisterValues::Vector {
        //             values,
        //             index: register_index,
        //             offset,
        //         } = &register.values
        //         else {
        //             panic!();
        //         };
        //
        //         // TODO:
        //     }
        // }
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
                    values,
                    index: register_index,
                    offset,
                } = &register.values
                else {
                    panic!();
                };

                let new_index = usize::try_from(register_index.saturating_sub(*offset))
                    .unwrap()
                    .clamp(0, values.len() - 1);

                if *index != new_index {
                    *index = new_index;

                    *target_scroll = *index as f32;
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
