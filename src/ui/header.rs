use macroquad::{
    input::{self, MouseButton},
    math::Vec2,
};

use super::{
    element::{Element, UpdateContext, UpdateResult, WindowFocusUse},
    window::{self, Window},
};

#[derive(Clone, Debug)]
pub struct Header {
    pub title: String,
}

impl Header {
    pub const HEIGHT: f32 = 35.0;
    pub const TITLE_HEIGHT: f32 = Window::BASE_TEXT_HEIGHT;
    pub const TITLE_OFFSET: Vec2 = Vec2::new(5.0, (Self::HEIGHT - Self::TITLE_HEIGHT) / 2.0);
}

impl Element for Header {
    fn height(&self) -> f32 {
        Self::HEIGHT
    }

    fn uses_window_focus(&self) -> WindowFocusUse {
        WindowFocusUse::Always
    }

    fn update(&mut self, UpdateContext { window, focus, .. }: UpdateContext) -> UpdateResult {
        let clicked = focus.mouse_hover && input::is_mouse_button_pressed(MouseButton::Left);

        let dragged = window.is_grabbed && input::is_mouse_button_down(MouseButton::Left);

        UpdateResult {
            grab_window: clicked || dragged,
            ..Default::default()
        }
    }

    fn draw(&mut self, UpdateContext { window, area, .. }: UpdateContext) {
        let [foreground, background] = if window.is_focused {
            [
                window.theme.darker_background_color,
                window.theme.accent_color,
            ]
        } else {
            [window.theme.accent_color, window.theme.background_color]
        };

        area.draw_rectangle(background);

        window::draw_text_with_size(
            &self.title,
            area.offset + Self::TITLE_OFFSET,
            Self::TITLE_HEIGHT,
            foreground,
        );
    }
}
