use macroquad::color::{Color, colors};

#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub accent_color: Color,
    pub background_color: Color,
    pub darker_background_color: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent_color: colors::WHITE,
            background_color: Color::from_hex(0x28283a),
            darker_background_color: Color::from_hex(0x101018),
        }
    }
}
