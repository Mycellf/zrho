use std::{fs, sync::LazyLock};

use macroquad::text::{self, Font};

pub mod text_editor;
pub mod window;

/// The width of each character should be 0.6 times the font size
pub static FONT: LazyLock<Font> = LazyLock::new(|| {
    text::load_ttf_font_from_bytes(&fs::read("assets/CommitMonoNerdFontMono-Regular.otf").unwrap())
        .unwrap()
});
