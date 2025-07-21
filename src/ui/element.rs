use std::{
    env,
    fmt::Debug,
    ops::{BitOr, BitOrAssign, Deref, DerefMut},
    sync::LazyLock,
};

use macroquad::{
    color::Color,
    math::Vec2,
    shapes,
    texture::{self, DrawTextureParams, Texture2D},
};

use super::window::WindowContext;

pub mod header;
pub mod scroll_bar;
pub mod space;
pub mod text_editor;

pub trait Element: Debug + 'static {
    #[must_use]
    fn height(&self) -> f32;

    #[must_use]
    fn uses_keyboard_focus(&self) -> bool {
        false
    }

    #[must_use]
    fn uses_window_focus(&self) -> WindowFocusUse {
        WindowFocusUse::Never
    }

    #[must_use]
    fn update(&mut self, context: UpdateContext) -> UpdateResult;

    fn draw(&mut self, context: UpdateContext);

    fn force_update(&mut self) {}
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct UpdateContext {
    pub window: WindowContext,
    pub area: DrawArea,
    pub focus: Focus,
}

impl UpdateContext {
    #[must_use]
    pub fn new(window: WindowContext, area: DrawArea, element: &ElementEntry) -> Self {
        let mut focus = element.focus;

        focus.keyboard &= window.is_focused;

        UpdateContext {
            window,
            area,
            focus,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Focus {
    pub mouse_grab: bool,
    pub mouse_hover: bool,
    pub keyboard: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum WindowFocusUse {
    Never,
    WithKeyboardFocus,
    Always,
}

impl WindowFocusUse {
    #[must_use]
    pub fn is_usesd(&self, keyboard_focus: bool) -> bool {
        match self {
            WindowFocusUse::Never => false,
            WindowFocusUse::WithKeyboardFocus => keyboard_focus,
            WindowFocusUse::Always => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UpdateResult {
    pub update_graphics: bool,
    pub grab_mouse: bool,
    pub grab_window: bool,
    pub take_keyboard_focus: bool,
}

impl BitOr for UpdateResult {
    type Output = UpdateResult;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            update_graphics: self.update_graphics | rhs.update_graphics,
            grab_mouse: self.grab_mouse | rhs.grab_mouse,
            grab_window: self.grab_window | rhs.grab_window,
            take_keyboard_focus: self.take_keyboard_focus | rhs.take_keyboard_focus,
        }
    }
}

impl BitOrAssign for UpdateResult {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct ElementEntry {
    pub inner: Box<dyn Element>,
    pub needs_update: bool,
    pub focus: Focus,
}

impl ElementEntry {
    /// Same as `Element::draw` except it draws a randomly colored box over everything if the
    /// `--rainbow-debug` flag is passed.
    pub fn draw(&mut self, context: UpdateContext) {
        self.inner.draw(context);

        context.area.draw_rainbow_debug();
    }

    pub fn new(element: impl Element) -> Self {
        Self {
            focus: Focus {
                keyboard: element.uses_keyboard_focus(),
                ..Default::default()
            },
            inner: Box::new(element),
            needs_update: true,
        }
    }
}

impl Deref for ElementEntry {
    type Target = dyn Element;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl DerefMut for ElementEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DrawArea {
    pub offset: Vec2,
    pub size: Vec2,
}

impl DrawArea {
    #[must_use]
    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x > self.offset.x
            && point.y > self.offset.y
            && point.x < self.offset.x + self.size.x
            && point.y < self.offset.y + self.size.y
    }

    #[must_use]
    pub fn clamp_point(&self, point: Vec2) -> Vec2 {
        Vec2::new(
            point.x.clamp(self.offset.x, self.offset.x + self.size.x),
            point.y.clamp(self.offset.y, self.offset.y + self.size.y),
        )
    }

    pub fn draw_rectangle(&self, color: Color) {
        shapes::draw_rectangle(
            self.offset.x,
            self.offset.y,
            self.size.x,
            self.size.y,
            color,
        )
    }

    pub fn draw_rectangle_inside(&self, mut location: Vec2, mut size: Vec2, color: Color) {
        if location.x < self.offset.x {
            size.x += location.x - self.offset.x;
            location.x = self.offset.x;
        }

        if location.x + size.x > self.offset.x + self.size.x {
            size.x = (self.offset.x + self.size.x) - location.x;
        }

        if location.y < self.offset.y {
            size.y += location.y - self.offset.y;
            location.y = self.offset.y;
        }

        if location.y + size.y > self.offset.y + self.size.y {
            size.y = (self.offset.y + self.size.y) - location.y;
        }

        if size.x > 0.0 && size.y > 0.0 {
            shapes::draw_rectangle(location.x, location.y, size.x, size.y, color)
        }
    }

    pub fn draw_rectangle_lines(&self, thickness: f32, color: Color) {
        shapes::draw_rectangle_lines(
            self.offset.x,
            self.offset.y,
            self.size.x,
            self.size.y,
            thickness,
            color,
        )
    }

    pub fn draw_rainbow_debug(&self) {
        static RAINBOW_DEBUG: LazyLock<bool> = LazyLock::new(|| {
            env::args()
                .skip(1)
                .any(|argument| argument == "--rainbow-debug")
        });

        if *RAINBOW_DEBUG {
            let color = Color::from_hex(macroquad::rand::gen_range(0, 0xffffff));

            self.draw_rectangle(Color { a: 0.25, ..color });
            self.draw_rectangle_lines(5.0, color);
        }
    }

    pub fn draw_texture(&self, texture: &Texture2D, color: Color) {
        texture::draw_texture_ex(
            texture,
            self.offset.x,
            self.offset.y,
            color,
            DrawTextureParams {
                dest_size: Some(self.size),
                ..Default::default()
            },
        );
    }
}
