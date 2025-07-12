use std::{fs, sync::LazyLock};

use macroquad::{
    camera::{self, Camera2D},
    color::{Color, colors},
    input::{self, MouseButton},
    math::Vec2,
    shapes,
    text::{self, Font, TextParams},
    texture::{self, DrawTextureParams, FilterMode, RenderTargetParams},
    window,
};

use crate::simulation::{
    computer::Computer,
    program::{Program, ProgramAssemblyError},
};

use super::text_editor::TextEditor;

pub const SCREEN_HEIGHT: f32 = 1000.0;
pub fn total_screen_width() -> f32 {
    window::screen_width() / scaling_factor()
}

pub fn scaling_factor() -> f32 {
    window::screen_height() / SCREEN_HEIGHT
}

/// The width of each character should be 0.6 times the font size
pub static FONT: LazyLock<Font> = LazyLock::new(|| {
    text::load_ttf_font_from_bytes(&fs::read("assets/CommitMonoNerdFontMono-Regular.otf").unwrap())
        .unwrap()
});

#[derive(Debug)]
pub struct EditorWindow {
    pub position: Vec2,
    pub size: Vec2,
    pub title: String,
    pub title_color: Color,

    pub grab_position: Option<Vec2>,
    pub is_focused: bool,

    pub text_editor: TextEditor,
    pub program: Result<Program, Vec<ProgramAssemblyError>>,

    pub camera: Camera2D,
    pub contents_updated: bool,
}

impl EditorWindow {
    pub const BACKGROUND_COLOR: Color = Color::from_hex(0x101018);
    pub const WINDOW_COLOR: Color = Color::from_hex(0x181824);
    pub const HEADER_COLOR: Color = Color::from_hex(0x202030);

    pub const RED: Color = Color::from_hex(0xff0000);
    pub const ORANGE: Color = Color::from_hex(0xff7f00);
    pub const BLUE: Color = Color::from_hex(0x007fff);

    pub const BORDER_WIDTH: f32 = 2.5;

    pub const TEXT_SIZE: f32 = 15.0;
    pub const TITLE_PADDING: f32 = 20.0;
    pub const TITLE_HEIGHT: f32 = Self::TEXT_SIZE + Self::TITLE_PADDING;

    pub fn new(
        position: Vec2,
        size: Vec2,
        title: String,
        title_color: Color,
        text_editor: TextEditor,
        target_computer: &Computer,
        sample_count: i32,
        scale: f32,
    ) -> EditorWindow {
        let grab_position = None;
        let is_focused = false;

        let program = Program::assemble_from(title.clone(), &text_editor.text, target_computer);

        let target_size = size * scale;

        let camera = Camera2D {
            zoom: -2.0 / size,
            offset: Vec2::new(1.0, 1.0),
            render_target: Some({
                let render_target = texture::render_target_ex(
                    target_size.x as u32,
                    target_size.y as u32,
                    RenderTargetParams {
                        sample_count,
                        depth: false,
                    },
                );
                render_target.texture.set_filter(FilterMode::Linear);
                render_target
            }),
            ..Default::default()
        };
        let window_updated = true;

        Self {
            position,
            size,
            title,
            title_color,

            grab_position,
            is_focused,

            text_editor,
            program,

            camera,
            contents_updated: window_updated,
        }
    }

    pub fn assemble_program(&mut self, target_computer: &Computer) {
        self.program =
            Program::assemble_from(self.title.clone(), &self.text_editor.text, target_computer);
    }

    pub fn update(&mut self, any_window_grabbed: bool, ordering: usize) -> bool {
        let is_focused = ordering == 0;
        self.contents_updated |= is_focused ^ self.is_focused;

        self.is_focused = is_focused;

        let mouse_position = Vec2::from(input::mouse_position());

        let is_clicked = self.is_point_within_bounds(mouse_position)
            && input::is_mouse_button_pressed(MouseButton::Left);

        if let Some(grab_position) = self.grab_position {
            self.position = mouse_position - grab_position;

            if !input::is_mouse_button_down(MouseButton::Left) {
                self.grab_position = None;
            }
        } else if is_clicked
            && !any_window_grabbed
            && input::is_mouse_button_down(MouseButton::Left)
            && self.is_point_within_title_bar(mouse_position)
        {
            self.grab_position = Some(mouse_position - self.position);
        }

        self.clamp_within_window_boundary();

        is_clicked
    }

    pub fn clamp_within_window_boundary(&mut self) {
        let scaling_factor = scaling_factor();

        if self.position.x + self.size.x * scaling_factor > window::screen_width() {
            self.position.x = window::screen_width() - self.size.x * scaling_factor;
        }

        if self.position.x < 0.0 {
            self.position.x = 0.0;
        }

        if self.position.y + self.size.y * scaling_factor > window::screen_height() {
            self.position.y = window::screen_height() - self.size.y * scaling_factor;
        }

        if self.position.y < 0.0 {
            self.position.y = 0.0;
        }
    }

    #[must_use]
    pub fn is_point_within_bounds(&self, point: Vec2) -> bool {
        let size = self.size * scaling_factor();

        point.x >= self.position.x
            && point.y >= self.position.y
            && point.x <= self.position.x + size.x
            && point.y <= self.position.y + size.y
    }

    #[must_use]
    pub fn is_point_within_title_bar(&self, point: Vec2) -> bool {
        point.x >= self.position.x
            && point.y >= self.position.y
            && point.x <= self.position.x + self.size.x * scaling_factor()
            && point.y <= self.position.y + Self::TITLE_HEIGHT * scaling_factor()
    }

    pub fn draw(&mut self) {
        if self.contents_updated {
            self.contents_updated = false;

            self.update_texture();
        }

        let scaling_factor = scaling_factor();
        let size = self.size * scaling_factor;

        shapes::draw_rectangle(
            self.position.x,
            self.position.y,
            size.x,
            size.y,
            Self::WINDOW_COLOR,
        );

        texture::draw_texture_ex(
            &self.camera.render_target.as_ref().unwrap().texture,
            self.position.x,
            self.position.y,
            colors::WHITE,
            DrawTextureParams {
                dest_size: Some(size),
                flip_x: true,
                flip_y: true,
                ..Default::default()
            },
        );
    }

    /// BUG: Black boxes can appear over text if a new scaling factor has just been selected
    pub fn update_texture(&self) {
        camera::push_camera_state();
        camera::set_camera(&self.camera);

        let (font_size, font_scale, _) = text::camera_font_scale(Self::TEXT_SIZE);

        shapes::draw_rectangle(0.0, 0.0, self.size.x, self.size.y, Self::WINDOW_COLOR);

        // Text
        self.text_editor.draw_all(
            Vec2::new(Self::BORDER_WIDTH + 5.0, Self::TITLE_HEIGHT),
            Self::TEXT_SIZE,
            1.0,
            1.0,
        );

        // Header
        let (text_color, background_color) = if self.is_focused {
            (Self::BACKGROUND_COLOR, self.title_color)
        } else {
            (self.title_color, Self::WINDOW_COLOR)
        };

        shapes::draw_rectangle_lines(
            0.0,
            0.0,
            self.size.x,
            self.size.y,
            Self::BORDER_WIDTH * 2.0,
            background_color,
        );

        shapes::draw_rectangle(0.0, 0.0, self.size.x, Self::TITLE_HEIGHT, background_color);

        text::draw_text_ex(
            &self.title,
            5.0,
            Self::TEXT_SIZE * 0.875 + Self::TITLE_PADDING / 2.0,
            TextParams {
                font: Some(&FONT),
                font_size,
                font_scale,
                font_scale_aspect: 1.0,
                rotation: 0.0,
                color: text_color,
            },
        );

        camera::pop_camera_state();
    }
}
