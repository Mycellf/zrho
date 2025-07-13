use macroquad::{
    camera::{self, Camera2D},
    color::{Color, colors},
    input::{self, MouseButton},
    math::Vec2,
    shapes,
    text::{self, TextParams},
    texture::{self, DrawTextureParams},
    window,
};

use crate::{
    interface::FONT,
    simulation::{
        computer::Computer,
        program::{Program, ProgramAssemblyError},
    },
};

use super::text_editor::TextEditor;

pub const SCREEN_HEIGHT: f32 = 1000.0;
#[must_use]
pub fn total_screen_width() -> f32 {
    window::screen_width() / scaling_factor()
}

#[must_use]
pub fn scaling_factor() -> f32 {
    window::screen_height() / SCREEN_HEIGHT
}

#[derive(Debug)]
pub struct EditorWindow {
    pub position: Vec2,
    pub proportional_position: Vec2,

    pub size: Vec2,
    pub title: String,
    pub title_color: Color,

    pub grab_position: Option<Vec2>,
    pub is_focused: bool,

    pub text_editor: TextEditor,
    pub scroll: f32,
    /// TODO: debug
    pub scroll_speed: f32,
    pub text_offset: f32,
    pub program: Result<Program, Vec<ProgramAssemblyError>>,

    pub camera: Camera2D,
    pub contents_updated: bool,
}

impl EditorWindow {
    pub const BACKGROUND_COLOR: Color = Color::from_hex(0x08080b);

    pub const EDITOR_BACKGROUND_COLOR: Color = Color::from_hex(0x101018);
    pub const WINDOW_COLOR: Color = Color::from_hex(0x181824);
    pub const HEADER_COLOR: Color = Color::from_hex(0x202030);

    pub const RED: Color = Color::from_hex(0xff0000);
    pub const ORANGE: Color = Color::from_hex(0xff7f00);
    pub const BLUE: Color = Color::from_hex(0x007fff);

    pub const BORDER_WIDTH: f32 = 2.5;

    pub const TEXT_SIZE: f32 = 15.0;
    pub const TITLE_PADDING: f32 = 20.0;
    pub const TITLE_HEIGHT: f32 = Self::TEXT_SIZE + Self::TITLE_PADDING;

    pub const TEXT_UPSCALING: u16 = 1;
    pub const RESOLUTION_UPSCALING: u16 = 4;

    pub fn new(
        proportional_position: Vec2,
        size: Vec2,
        title: String,
        title_color: Color,
        text_editor: TextEditor,
        target_computer: &Computer,
    ) -> EditorWindow {
        let position = Self::position_from_proportionally(proportional_position, size);

        let grab_position = None;
        let is_focused = false;

        let scroll = 0.0;
        let text_offset = 0.0;
        let program = Program::assemble_from(title.clone(), &text_editor.text, target_computer);

        let target_size = size * Self::RESOLUTION_UPSCALING as f32;

        let camera = Camera2D {
            zoom: -2.0 / size,
            offset: Vec2::new(1.0, 1.0),
            render_target: Some(texture::render_target(
                target_size.x as u32,
                target_size.y as u32,
            )),
            ..Default::default()
        };
        let contents_updated = true;

        Self {
            position,
            proportional_position,

            size,
            title,
            title_color,

            grab_position,
            is_focused,

            text_editor,
            scroll,
            scroll_speed: 5.0,
            text_offset,
            program,

            camera,
            contents_updated,
        }
    }

    pub fn assemble_program(&mut self, target_computer: &Computer) {
        self.program =
            Program::assemble_from(self.title.clone(), &self.text_editor.text, target_computer);
    }

    pub fn update(&mut self, any_window_grabbed: bool) -> bool {
        let mouse_position = Vec2::from(input::mouse_position());

        let is_clicked = self.is_point_within_bounds(mouse_position)
            && input::is_mouse_button_pressed(MouseButton::Left);

        if let Some(grab_position) = self.grab_position {
            if input::is_mouse_button_down(MouseButton::Left) {
                self.position = mouse_position - grab_position;
                self.clamp_within_window_boundary();

                self.proportional_position =
                    Self::proportional_position_from(self.position, self.size);
            } else {
                self.grab_position = None;
            }
        } else if is_clicked
            && !any_window_grabbed
            && input::is_mouse_button_down(MouseButton::Left)
            && self.is_point_within_title_bar(mouse_position)
        {
            self.grab_position = Some(mouse_position - self.position);
        } else {
            self.position =
                Self::position_from_proportionally(self.proportional_position, self.size);
        }

        self.scroll += macroquad::time::get_frame_time() * self.scroll_speed;
        match self.scroll_speed {
            ..0.0 => {
                if self.scroll <= 0.0 {
                    self.scroll_speed *= -1.0;
                    self.scroll = 0.0;
                }
            }
            0.0.. => {
                let maximum_scroll = (self.text_editor.num_lines() - 1) as f32;

                if self.scroll >= maximum_scroll {
                    self.scroll_speed *= -1.0;
                    self.scroll = maximum_scroll
                }
            }
            _ => unreachable!(),
        }
        self.contents_updated = true;

        self.text_offset = (self.scroll.floor() - self.scroll) * Self::TEXT_SIZE;

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

    #[must_use]
    pub fn is_point_within_editor(&self, point: Vec2) -> bool {
        point.x >= self.position.x
            && point.y >= self.position.y + Self::TITLE_HEIGHT * scaling_factor()
            && point.x <= self.position.x + self.size.x * scaling_factor()
            && point.y <= self.position.y + self.size.y * scaling_factor()
    }

    #[must_use]
    pub fn height_of_editor(&self) -> f32 {
        self.size.y - Self::TITLE_HEIGHT - Self::BORDER_WIDTH
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
            Self::EDITOR_BACKGROUND_COLOR,
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

    pub fn update_texture(&self) {
        camera::push_camera_state();
        camera::set_camera(&self.camera);

        shapes::draw_rectangle(
            0.0,
            0.0,
            self.size.x,
            self.size.y,
            Self::EDITOR_BACKGROUND_COLOR,
        );

        // Text
        let start_line = self.scroll.floor() as usize;
        let end_line = (self.scroll + self.height_of_editor() / Self::TEXT_SIZE).ceil() as usize;

        self.text_editor.draw_range(
            start_line..end_line,
            Vec2::new(
                Self::BORDER_WIDTH + 5.0,
                Self::TITLE_HEIGHT + self.text_offset,
            ),
            Self::TEXT_SIZE,
            1.0,
            1.0,
        );

        // Header and outline
        let (text_color, background_color) = if self.is_focused {
            (Self::EDITOR_BACKGROUND_COLOR, self.title_color)
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
                color: text_color,
                ..Self::text_params_with_size(Self::TEXT_SIZE)
            },
        );

        // Scroll bar
        let scroll_bar_width = 5.0;
        let scroll_bar_height = self.height_of_editor()
            / (self.text_editor.num_lines() as f32 * Self::TEXT_SIZE + self.height_of_editor()
                - Self::TEXT_SIZE);

        if scroll_bar_height < 1.0 {
            let scroll_bar_height = (scroll_bar_height * self.height_of_editor()).max(20.0);

            let scroll_bar_position = (self.height_of_editor() - scroll_bar_height)
                * (self.scroll / (self.text_editor.num_lines() - 1) as f32);

            shapes::draw_rectangle(
                self.size.x - scroll_bar_width,
                Self::TITLE_HEIGHT + scroll_bar_position,
                scroll_bar_width,
                scroll_bar_height,
                colors::LIGHTGRAY,
            );
        }

        camera::pop_camera_state();
    }

    pub fn text_params_with_size(text_size: f32) -> TextParams<'static> {
        TextParams {
            font: Some(&FONT),
            font_size: (text_size * Self::TEXT_UPSCALING as f32) as u16,
            font_scale: 1.0 / Self::TEXT_UPSCALING as f32,
            font_scale_aspect: 1.0,
            rotation: 0.0,
            color: colors::WHITE,
        }
    }

    #[must_use]
    pub fn proportional_position_from(position: Vec2, size: Vec2) -> Vec2 {
        let size = size * scaling_factor();

        let maximum_position = Vec2::new(window::screen_width(), window::screen_height()) - size;

        fn safe_divide(position: f32, maximum_position: f32) -> f32 {
            if maximum_position > 0.0 {
                position / maximum_position
            } else {
                0.0
            }
        }

        Vec2::new(
            safe_divide(position.x, maximum_position.x),
            safe_divide(position.y, maximum_position.y),
        )
    }

    #[must_use]
    pub fn position_from_proportionally(proportional_position: Vec2, size: Vec2) -> Vec2 {
        let size = size * scaling_factor();

        let position = proportional_position
            * (Vec2::new(window::screen_width(), window::screen_height()) - size);

        let dpi_scale = window::miniquad::window::dpi_scale();

        (position.max(Vec2::ZERO) * dpi_scale).round() / dpi_scale
    }
}
