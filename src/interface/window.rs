use std::{fs, sync::LazyLock};

use macroquad::{
    camera::{self, Camera2D},
    color::{Color, colors},
    input::{self, MouseButton},
    math::Vec2,
    shapes,
    text::{self, Font, TextParams},
    texture::{self, DrawTextureParams, FilterMode},
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

    pub text_editor: TextEditor,
    pub program: Result<Program, Vec<ProgramAssemblyError>>,

    pub camera: Camera2D,
    pub window_updated: bool,
}

impl EditorWindow {
    pub const BACKGROUND_COLOR: Color = Color::from_hex(0x101018);
    pub const WINDOW_COLOR: Color = Color::from_hex(0x181824);

    pub const RED: Color = Color::from_hex(0xff0000);
    pub const BLUE: Color = Color::from_hex(0x007fff);

    pub const BORDER_WIDTH: f32 = 5.0;

    pub const TEXT_SIZE: f32 = 15.0;
    pub const TITLE_HEIGHT: f32 = 20.0;

    pub fn new(
        position: Vec2,
        size: Vec2,
        title: String,
        title_color: Color,
        text_editor: TextEditor,
        target_computer: &Computer,
    ) -> EditorWindow {
        let grab_position = None;

        let program = Program::assemble_from(title.clone(), &text_editor.text, target_computer);

        let content_size = size - Self::BORDER_WIDTH * 2.0;
        let target_size = content_size * 4.0;

        let camera = Camera2D {
            zoom: -2.0 / content_size,
            offset: Vec2::new(1.0, 1.0),
            render_target: Some({
                let render_target =
                    texture::render_target(target_size.x as u32, target_size.y as u32);
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

            text_editor,
            program,

            camera,
            window_updated,
        }
    }

    pub fn assemble_program(&mut self, target_computer: &Computer) {
        self.program =
            Program::assemble_from(self.title.clone(), &self.text_editor.text, target_computer);
    }

    pub fn update(&mut self, any_window_grabbed: bool) {
        let mouse_position = Vec2::from(input::mouse_position());

        if let Some(grab_position) = self.grab_position {
            self.position = mouse_position - grab_position;

            if !input::is_mouse_button_down(MouseButton::Left) {
                self.grab_position = None;
            }
        } else if !any_window_grabbed
            && input::is_mouse_button_pressed(MouseButton::Left) // Trackpad digital click will only
            && input::is_mouse_button_down(MouseButton::Left)    // trigger is_mouse_button_pressed
            && self.is_point_within_bounds(mouse_position)
        {
            self.grab_position = Some(mouse_position - self.position);
        }

        self.clamp_within_window_boundary();
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

    pub fn draw(&mut self) {
        if self.window_updated {
            self.window_updated = false;

            self.update_texture();
        }

        let scaling_factor = scaling_factor();
        let full_size = self.size * scaling_factor;
        let border_width = Self::BORDER_WIDTH * scaling_factor;

        texture::draw_texture_ex(
            &self.camera.render_target.as_ref().unwrap().texture,
            self.position.x + border_width,
            self.position.y + border_width,
            colors::WHITE,
            DrawTextureParams {
                dest_size: Some(full_size - border_width),
                flip_x: true,
                flip_y: true,
                ..Default::default()
            },
        );

        shapes::draw_rectangle_lines(
            self.position.x,
            self.position.y,
            full_size.x,
            full_size.y,
            border_width * 2.0,
            self.title_color,
        );
    }

    pub fn update_texture(&self) {
        camera::push_camera_state();
        camera::set_camera(&self.camera);

        let (font_size, font_scale, _) = text::camera_font_scale(Self::TEXT_SIZE);

        shapes::draw_rectangle(0.0, 0.0, self.size.x, self.size.y, Self::WINDOW_COLOR);

        text::draw_text_ex(
            &self.title,
            5.0,
            Self::TEXT_SIZE + Self::TITLE_HEIGHT / 2.0,
            TextParams {
                font: Some(&FONT),
                font_size,
                font_scale,
                font_scale_aspect: 1.0,
                rotation: 0.0,
                color: self.title_color,
            },
        );

        self.text_editor.draw_all(
            Vec2::new(5.0, 5.0 + Self::TEXT_SIZE + Self::TITLE_HEIGHT),
            Self::TEXT_SIZE,
            1.0,
            1.0,
        );

        camera::pop_camera_state();
    }
}
