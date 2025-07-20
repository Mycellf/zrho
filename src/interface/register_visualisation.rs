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
    pub const COLUMN_WIDTH: f32 = RegisterVisualisation::WIDTH + Self::HORIZONTAL_SPACING;

    pub fn new(computer: &Computer, num_rows: usize) -> Self {
        let mut scalars = Vec::new();
        let mut indexes = Vec::new();
        let mut vectors = Vec::new();

        for (i, register) in computer.registers.registers.iter().enumerate() {
            let Some(register) = register else { continue };

            let visualisation = RegisterVisualisation::new(i.try_into().unwrap(), register);

            let visualisation_set = match visualisation.value_visualisation {
                ValueVisualisation::Scalar => &mut scalars,
                ValueVisualisation::Index => &mut indexes,
                ValueVisualisation::Vector { .. } => &mut vectors,
            };

            visualisation_set.push(visualisation);
        }

        let mut visualisations = Vec::new();

        scalars.sort_by_key(|&RegisterVisualisation { register, .. }| {
            computer::ordering_of_register(register)
        });

        let vectors_start = scalars.len().div_ceil(num_rows);

        for (i, mut visualisation) in scalars.into_iter().enumerate() {
            visualisation.position = RegisterVisualisationPosition {
                column: i / num_rows,
                row: i % num_rows,
            };

            visualisations.push(visualisation);
        }

        for (visualisation, column) in vectors.iter_mut().zip(vectors_start..) {
            visualisation.position = RegisterVisualisationPosition {
                column,
                row: computer
                    .registers
                    .get(visualisation.register)
                    .unwrap()
                    .indexed_by
                    .is_some() as usize,
            };

            visualisations.push(*visualisation);
        }

        for mut visualisation in indexes.into_iter() {
            let target_register = computer
                .registers
                .get(visualisation.register)
                .unwrap()
                .indexes_array
                .unwrap();

            let target_index = vectors
                .iter()
                .position(|&RegisterVisualisation { register, .. }| target_register == register)
                .unwrap();

            visualisation.position = RegisterVisualisationPosition {
                column: vectors[target_index].position.column,
                row: 0,
            };

            visualisations.push(visualisation);
        }

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
        for register_visualisation in &self.visualisations {
            register_visualisation.draw_background_at(
                location + register_visualisation.offset(),
                computer
                    .registers
                    .get(register_visualisation.register)
                    .unwrap(),
                color,
            );
        }

        // PERFORMANCE: Interleaving text and shape drawing is extremely slow
        for register_visualisation in &self.visualisations {
            register_visualisation.draw_text_at(
                location + register_visualisation.offset(),
                computer
                    .registers
                    .get(register_visualisation.register)
                    .unwrap(),
            );
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RegisterVisualisationPosition {
    pub column: usize,
    pub row: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct RegisterVisualisation {
    pub register: u32,
    pub value_visualisation: ValueVisualisation,
    pub position: RegisterVisualisationPosition,
}

impl RegisterVisualisation {
    pub const WIDTH: f32 = EditorWindow::TEXT_WIDTH * 5.0;
    pub const BASE_HEIGHT: f32 = EditorWindow::TEXT_SIZE * 2.0;
    pub const HEIGHT: f32 = Self::BASE_HEIGHT + RegisterVisualisationLayout::VERTICAL_SPACING;

    pub fn offset(&self) -> Vec2 {
        let nudge_up = self.value_visualisation.is_vector() && self.position.row % 2 == 1;

        Vec2::new(
            self.position.column as f32 * RegisterVisualisationLayout::COLUMN_WIDTH,
            self.position.row as f32 * RegisterVisualisation::HEIGHT
                - nudge_up as usize as f32 * RegisterVisualisationLayout::VERTICAL_SPACING,
        )
    }

    #[must_use]
    pub fn new(index: u32, register: &Register) -> Self {
        Self {
            register: index,
            value_visualisation: match register.values {
                RegisterValues::Scalar(..) => {
                    if register.indexes_array.is_none() {
                        ValueVisualisation::Scalar
                    } else {
                        ValueVisualisation::Index
                    }
                }
                RegisterValues::Vector { .. } => ValueVisualisation::Vector {
                    index: 0,
                    scroll: 0.0,
                    target_scroll: 0.0,
                },
            },
            position: RegisterVisualisationPosition::default(),
        }
    }

    pub fn update(&mut self, register: &Register) {
        self.value_visualisation.update(register);
    }

    pub fn draw_background_at(&self, location: Vec2, register: &Register, title_color: Color) {
        let width = if register.block_time > 0 {
            EditorWindow::TEXT_WIDTH * 2.0
        } else {
            Self::WIDTH
        };

        // Name
        shapes::draw_rectangle(
            location.x,
            location.y,
            width,
            EditorWindow::TEXT_SIZE,
            title_color,
        );

        if register.block_time > 0 {
            // Block time
            let width = Self::WIDTH - width - EditorWindow::BORDER_WIDTH;

            shapes::draw_rectangle(
                location.x + Self::WIDTH - width,
                location.y,
                width,
                EditorWindow::TEXT_SIZE,
                EditorWindow::EDITOR_BACKGROUND_COLOR,
            );
        }

        // Value
        let background_color = match register.value() {
            Ok(_) => EditorWindow::EDITOR_BACKGROUND_COLOR,
            Err(_) => Color::from_hex(0xff0000),
        };

        let location = location + Vec2::new(0.0, EditorWindow::TEXT_SIZE);

        shapes::draw_rectangle(
            location.x,
            location.y,
            Self::WIDTH,
            EditorWindow::TEXT_SIZE,
            background_color,
        );

        shapes::draw_rectangle_lines(
            location.x,
            location.y,
            width,
            EditorWindow::TEXT_SIZE,
            2.0,
            title_color,
        );
    }

    pub fn draw_text_at(&self, location: Vec2, register: &Register) {
        let name = computer::name_of_register(self.register).unwrap();

        let width = if register.block_time > 0 {
            EditorWindow::TEXT_WIDTH * 2.0
        } else {
            Self::WIDTH
        };

        // Name
        draw_centered_text(
            &name.to_string(),
            location,
            width,
            EditorWindow::EDITOR_BACKGROUND_COLOR,
        );

        if register.block_time > 0 {
            // Block time
            let width = Self::WIDTH - width - EditorWindow::BORDER_WIDTH;

            draw_centered_text(
                &register.block_time.to_string(),
                location + Vec2::new(Self::WIDTH - width, 0.0),
                width,
                colors::WHITE,
            );
        }

        // Value
        let (value, foreground_color) = match register.value() {
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
                EditorWindow::EDITOR_BACKGROUND_COLOR,
            ),
        };

        let location = location + Vec2::new(0.0, EditorWindow::TEXT_SIZE);

        draw_centered_text(&value, location, Self::WIDTH, foreground_color);
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
