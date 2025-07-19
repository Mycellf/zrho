use std::f32::consts::FRAC_1_SQRT_2;

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
    pub const HORIZONTAL_SPACING: f32 = EditorWindow::TEXT_WIDTH * 1.5;
    pub const VERTICAL_SPACING: f32 = EditorWindow::TEXT_SIZE / 2.0;
    pub const COLUMN_WIDTH: f32 = RegisterVisualisation::NAME_WIDTH + Self::HORIZONTAL_SPACING;

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

        for register_visualisation in &self.visualisations {
            let column = computer::column_of_register(register_visualisation.register);

            register_visualisation.draw_at(
                location + Vec2::new(column as f32 * Self::COLUMN_WIDTH, column_height[column]),
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
    pub const HEIGHT: f32 =
        EditorWindow::TEXT_SIZE * 2.0 + RegisterVisualisationLayout::VERTICAL_SPACING;

    #[must_use]
    pub fn new(index: u32, register: &Register) -> Self {
        Self {
            register: index,
            value_visualisation: match register.values {
                RegisterValues::Scalar(..) => {
                    if register.indexes_array.is_some() {
                        ValueVisualisation::Index
                    } else {
                        ValueVisualisation::Scalar
                    }
                }
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
        if self.value_visualisation.is_index() {
            let offset = Vec2::new(Self::NAME_WIDTH, EditorWindow::TEXT_SIZE / 2.0);

            let start = location + offset;
            let end = location
                + offset
                + Vec2::new(RegisterVisualisationLayout::HORIZONTAL_SPACING * 0.8, 0.0);

            draw_arrow(start, end, 2.0, 7.5, title_color);
        }

        let name = computer::name_of_register(self.register).unwrap();

        let seperation = 2.0;

        let width = if register.block_time > 0 {
            (Self::NAME_WIDTH - seperation) * 0.4
        } else {
            Self::NAME_WIDTH
        };

        // Name
        shapes::draw_rectangle(
            location.x,
            location.y,
            width,
            EditorWindow::TEXT_SIZE,
            title_color,
        );

        draw_centered_text(
            &name.to_string(),
            location,
            width,
            EditorWindow::EDITOR_BACKGROUND_COLOR,
        );

        if register.block_time > 0 {
            // Block time
            let width = Self::NAME_WIDTH - width - seperation;

            shapes::draw_rectangle(
                location.x + Self::NAME_WIDTH - width,
                location.y,
                width,
                EditorWindow::TEXT_SIZE,
                Color::from_hex(0xff0000),
            );

            draw_centered_text(
                &register.block_time.to_string(),
                location + Vec2::new(Self::NAME_WIDTH - width, 0.0),
                width,
                EditorWindow::EDITOR_BACKGROUND_COLOR,
            );
        }

        // Value
        let (value, color) = match register.value() {
            Ok(value) => (value.to_string(), colors::WHITE),
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
                Color::from_hex(0xff0000),
            ),
        };

        draw_centered_text(
            &value,
            location + Vec2::new(0.0, EditorWindow::TEXT_SIZE),
            Self::NAME_WIDTH,
            color,
        );

        // match self.value_visualisation {
        //     ValueVisualisation::Scalar => {
        //         assert!(register.values.is_scalar());
        //         assert!(register.indexes_array.is_none());
        //     }
        //     ValueVisualisation::Index => {
        //         assert!(register.values.is_scalar());
        //         assert!(register.indexes_array.is_some());
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
}

#[derive(Clone, Copy, Debug)]
pub enum ValueVisualisation {
    Scalar,
    Index,
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
                assert!(register.values.is_scalar());
                assert!(register.indexes_array.is_none());
            }
            ValueVisualisation::Index => {
                assert!(register.values.is_scalar());
                assert!(register.indexes_array.is_some());
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
                    .unwrap_or(0)
                    .min(values.len() - 1);

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

    /// Returns `true` if the value visualisation is [`Scalar`].
    ///
    /// [`Scalar`]: ValueVisualisation::Scalar
    #[must_use]
    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar)
    }

    /// Returns `true` if the value visualisation is [`Index`].
    ///
    /// [`Index`]: ValueVisualisation::Index
    #[must_use]
    pub fn is_index(&self) -> bool {
        matches!(self, Self::Index)
    }

    /// Returns `true` if the value visualisation is [`Vector`].
    ///
    /// [`Vector`]: ValueVisualisation::Vector
    #[must_use]
    pub fn is_vector(&self) -> bool {
        matches!(self, Self::Vector { .. })
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

pub fn draw_arrow(start: Vec2, end: Vec2, thickness: f32, tip_size: f32, color: Color) {
    const ANGLE_A: Vec2 = Vec2::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2);
    const ANGLE_B: Vec2 = Vec2::new(-FRAC_1_SQRT_2, -FRAC_1_SQRT_2);

    let Some(direction) = (end - start).try_normalize() else {
        return;
    };

    shapes::draw_line(start.x, start.y, end.x, end.y, thickness, color);

    for angle in [ANGLE_A, ANGLE_B] {
        let direction = direction.rotate(angle);

        let start = end - direction * thickness / 2.0;

        let end = start + direction * tip_size;

        shapes::draw_line(start.x, start.y, end.x, end.y, thickness, color);
    }
}
