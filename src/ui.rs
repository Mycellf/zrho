use std::sync::{LazyLock, Mutex};

use macroquad::{
    color::Color,
    input::{self, KeyCode},
    math::Vec2,
    text::{self, Font},
};

pub mod element;
pub mod header;
pub mod scroll_bar;
pub mod space;
pub mod text_editor;
pub mod text_editor_operations;
pub mod theme;
pub mod window;

pub mod colors {
    use macroquad::color::Color;

    pub const RED: Color = Color::from_hex(0xff0000);
    pub const ORANGE: Color = Color::from_hex(0xff7f00);
    pub const YELLOW: Color = Color::from_hex(0xffef00);
    pub const GREEN: Color = Color::from_hex(0x00ff7f);
    pub const TEAL: Color = Color::from_hex(0x00efff);
    pub const BLUE: Color = Color::from_hex(0x007fff);
    pub const PURPLE: Color = Color::from_hex(0x7f00ff);
    pub const FUSCHIA: Color = Color::from_hex(0xff007f);
}

pub const SCREEN_HEIGHT: f32 = 1000.0;

#[must_use]
pub fn screen_width() -> f32 {
    macroquad::window::screen_width() / scaling_factor()
}

#[must_use]
pub fn scaling_factor() -> f32 {
    macroquad::window::screen_height() / SCREEN_HEIGHT
}

#[must_use]
pub fn mouse_position() -> Vec2 {
    Vec2::from(input::mouse_position()) / scaling_factor()
}

/// Consumes all typed characters and returns them in the correct order
pub fn get_chars_typed() -> impl Iterator<Item = char> {
    let mut characters = Vec::new();

    while let Some(character) = input::get_char_pressed() {
        characters.push(character);
    }

    // Typed characters are returned backwards by macroquad for some reason
    characters.into_iter().rev()
}

pub fn clear_chars_typed() {
    while input::get_char_pressed().is_some() {}
}

/// The width of each character should be 0.6 times the font size
pub static FONT: LazyLock<Font> = LazyLock::new(|| {
    text::load_ttf_font_from_bytes(include_bytes!("../assets/CommitMono-400-Regular.otf")).unwrap()
});

pub const FONT_ASPECT: f32 = 0.6;

pub const FONT_VERTICAL_OFFSET: f32 = 0.875;

#[must_use]
pub fn exp_decay_cutoff(a: f32, b: f32, decay: f32, dt: f32, cutoff: f32) -> (f32, bool) {
    if (a - b).abs() < cutoff {
        (b, true)
    } else {
        (exp_decay(a, b, decay, dt), false)
    }
}

/// CREDIT: Freya Holmér: <https://www.youtube.com/watch?v=LSNQuFEDOyQ>
#[must_use]
pub fn exp_decay(a: f32, b: f32, decay: f32, dt: f32) -> f32 {
    b + (a - b) * (-decay * dt).exp()
}

#[must_use]
pub const fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[must_use]
pub const fn color_lerp(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: lerp(a.r, b.r, t),
        g: lerp(a.g, b.g, t),
        b: lerp(a.b, b.b, t),
        a: lerp(a.a, b.a, t),
    }
}

static KEY_REPEATS: Mutex<KeyRepeats> = Mutex::new(KeyRepeats::DEFAULT);

pub fn is_key_typed(key: KeyCode) -> bool {
    let key_repeats = &mut *KEY_REPEATS.lock().unwrap();

    if input::is_key_pressed(key) {
        key_repeats.set_key(key);

        true
    } else {
        key_repeats.key == Some(key)
    }
}

pub fn update_key_repeats() {
    KEY_REPEATS.lock().unwrap().update();
}

/// HACK: This exists because macroquad won't give key repeats for the navigation keys
#[derive(Clone, Copy, Debug)]
struct KeyRepeats {
    pub delay: f32,
    pub interval: f32,
    pub state: Option<(KeyCode, f32)>,
    pub key: Option<KeyCode>,
}

impl KeyRepeats {
    pub const DEFAULT: Self = Self {
        delay: 0.5,
        interval: 0.03,
        state: None,
        key: None,
    };

    pub const CONTROL_CHARACTERS: [KeyCode; 6] = [
        KeyCode::LeftShift,
        KeyCode::RightShift,
        KeyCode::LeftControl,
        KeyCode::RightControl,
        KeyCode::LeftAlt,
        KeyCode::RightAlt,
    ];

    pub fn update(&mut self) {
        self.key = if let &mut Some((key_code, ref mut time)) = &mut self.state {
            if !input::get_keys_pressed()
                .into_iter()
                .all(|key_code| Self::CONTROL_CHARACTERS.contains(&key_code))
            {
                self.state = None;

                None
            } else if input::is_key_down(key_code) {
                *time -= macroquad::time::get_frame_time();

                (*time <= 0.0).then(|| {
                    *time = (*time + self.interval).max(0.0);

                    key_code
                })
            } else {
                self.state = None;

                None
            }
        } else {
            None
        };
    }

    pub fn set_key(&mut self, key_code: KeyCode) {
        if let Some((previous_key_code, _)) = self.state {
            if key_code == previous_key_code {
                return;
            }
        }

        self.state = Some((key_code, self.delay));
    }
}

impl Default for KeyRepeats {
    fn default() -> Self {
        Self::DEFAULT
    }
}
