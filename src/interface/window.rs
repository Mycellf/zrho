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

/// The width of each character should be 0.6 times the font size
pub static FONT: LazyLock<Font> = LazyLock::new(|| {
    text::load_ttf_font_from_bytes(&fs::read("assets/CommitMonoNerdFontMono-Regular.otf").unwrap())
        .unwrap()
});

#[derive(Debug)]
pub struct EditorWindow {
    pub position: Vec2,
    pub size: Vec2,
    pub name: String,

    pub grab_position: Option<Vec2>,

    pub text_editor: TextEditor,
    pub program: Result<Program, Vec<ProgramAssemblyError>>,

    pub camera: Camera2D,
}

impl EditorWindow {
    pub const BACKGROUND_COLOR: Color = Color::from_hex(0x202030);
    pub const TEXT_COLOR: Color = Color::from_hex(0xff0000);

    pub const TEXT_SIZE: f32 = 10.0;
    pub const TITLE_HEIGHT: f32 = 15.0;

    pub fn new(
        position: Vec2,
        size: Vec2,
        name: String,
        text_editor: TextEditor,
        target_computer: &Computer,
    ) -> EditorWindow {
        let grab_position = None;

        let program = Program::assemble_from(name.clone(), &text_editor.text, target_computer);
        let scale = window::screen_dpi_scale() as u32 * 2;
        let camera = Camera2D {
            zoom: -2.0 / size,
            offset: Vec2::new(1.0, 1.0),
            render_target: Some({
                let render_target =
                    texture::render_target(size.x as u32 * scale, size.y as u32 * scale);
                render_target.texture.set_filter(FilterMode::Linear);
                render_target
            }),
            ..Default::default()
        };

        Self {
            position,
            size,
            name,

            grab_position,

            text_editor,
            program,

            camera,
        }
    }

    pub fn assemble_program(&mut self, target_computer: &Computer) {
        self.program =
            Program::assemble_from(self.name.clone(), &self.text_editor.text, target_computer);
    }

    pub fn update(&mut self, any_window_grabbed: bool) {
        let mouse_position = Vec2::from(input::mouse_position());

        if let Some(grab_position) = self.grab_position {
            self.position = mouse_position - grab_position;

            if !input::is_mouse_button_down(MouseButton::Left) {
                self.grab_position = None;
            }
        } else if !any_window_grabbed
            && input::is_mouse_button_down(MouseButton::Left)
            && self.is_point_within_bounds(mouse_position)
        {
            self.grab_position = Some(mouse_position - self.position);
        }

        self.clamp_within_window_boundary();
    }

    pub fn clamp_within_window_boundary(&mut self) {
        if self.position.x + self.size.x > window::screen_width() {
            self.position.x = window::screen_width() - self.size.x;
        }

        if self.position.x < 0.0 {
            self.position.x = 0.0;
        }

        if self.position.y + self.size.y > window::screen_height() {
            self.position.y = window::screen_height() - self.size.y;
        }

        if self.position.y < 0.0 {
            self.position.y = 0.0;
        }
    }

    #[must_use]
    pub fn is_point_within_bounds(&self, point: Vec2) -> bool {
        point.x >= self.position.x
            && point.y >= self.position.y
            && point.x <= self.position.x + self.size.x
            && point.y <= self.position.y + self.size.y
    }

    pub fn draw(&self) {
        camera::push_camera_state();
        camera::set_camera(&self.camera);

        let (font_size, font_scale, _) = text::camera_font_scale(Self::TEXT_SIZE);

        shapes::draw_rectangle(0.0, 0.0, self.size.x, self.size.y, Self::BACKGROUND_COLOR);

        text::draw_text_ex(
            &self.name,
            0.0,
            0.0 + Self::TEXT_SIZE,
            TextParams {
                font: Some(&FONT),
                font_size,
                font_scale,
                font_scale_aspect: 1.0,
                rotation: 0.0,
                color: Self::TEXT_COLOR,
            },
        );

        self.text_editor.draw_all(
            Vec2::Y * (Self::TEXT_SIZE + Self::TITLE_HEIGHT),
            Self::TEXT_SIZE,
            1.0,
            1.0,
        );

        camera::pop_camera_state();

        texture::draw_texture_ex(
            &self.camera.render_target.as_ref().unwrap().texture,
            self.position.x,
            self.position.y,
            colors::WHITE,
            DrawTextureParams {
                dest_size: Some(self.size),
                flip_x: true,
                flip_y: true,
                ..Default::default()
            },
        );
    }
}
