use std::sync::LazyLock;

use macroquad::text::{self, Font};

pub mod editor_window;
pub mod register_visualisation;
pub mod text_editor;

/// The width of each character should be 0.6 times the font size
pub static FONT: LazyLock<Font> = LazyLock::new(|| {
    text::load_ttf_font_from_bytes(include_bytes!("../assets/CommitMono-400-Regular.otf")).unwrap()
});

pub const FONT_ASPECT: f32 = 0.6;
