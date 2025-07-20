use macroquad::{color::Color, math::Vec2, shapes};

use crate::interface::editor_window::EditorWindow;

#[derive(Clone, Copy, Debug, Default)]
pub struct SimulationControlPanel {
    pub mouse_position: Option<usize>,
}

impl SimulationControlPanel {
    pub const BUTTON_SIZE: f32 = EditorWindow::TEXT_SIZE * 2.0;
    pub const BUTTON_DISTANCE: f32 = 2.5;
    pub const BUTTON_SPACING: f32 = Self::BUTTON_SIZE + Self::BUTTON_DISTANCE;

    pub const WIDTH: f32 = Self::NUM_BUTTONS as f32 * Self::BUTTON_SPACING + Self::BUTTON_DISTANCE;
    pub const HEIGHT: f32 = Self::BUTTON_SIZE + 2.0 * Self::BUTTON_DISTANCE;

    pub const NUM_BUTTONS: usize = 5;

    pub fn update_mouse_position(&mut self, relative_position: Vec2) -> bool {
        let new_position = if relative_position.y <= Self::BUTTON_DISTANCE
            || relative_position.y >= Self::BUTTON_DISTANCE + Self::BUTTON_SIZE
            || relative_position.x <= Self::BUTTON_DISTANCE
            || relative_position.x >= Self::BUTTON_SPACING * Self::NUM_BUTTONS as f32
        {
            None
        } else {
            let button_position = relative_position.x % Self::BUTTON_SPACING;
            let button = (relative_position.x / Self::BUTTON_SPACING).floor() as usize;

            if button_position <= Self::BUTTON_DISTANCE {
                None
            } else {
                Some(button)
            }
        };

        let updated = self.mouse_position != new_position;

        self.mouse_position = new_position;

        updated
    }

    pub fn draw_at(&self, location: Vec2, color: Color, invert: bool) {
        for button in 0..Self::NUM_BUTTONS {
            let selected = self.mouse_position == Some(button);

            if !selected {
                shapes::draw_rectangle(
                    location.x + Self::BUTTON_DISTANCE + Self::BUTTON_SPACING * button as f32,
                    location.y + Self::BUTTON_DISTANCE,
                    Self::BUTTON_SIZE,
                    Self::BUTTON_SIZE,
                    if invert {
                        EditorWindow::EDITOR_BACKGROUND_COLOR
                    } else {
                        color
                    },
                );
            }
        }
    }
}
