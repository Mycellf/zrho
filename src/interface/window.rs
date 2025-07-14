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
    pub scroll_bar: Option<ScrollBar>,
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

    pub const RESOLUTION_UPSCALING: u16 = 4;

    pub const WINDOW_PADDING: f32 = 10.0;
    pub const ELEMENT_PADDING: f32 = 5.0;

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
        let scroll_bar = None;
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
            scroll_bar,
            text_offset,
            program,

            camera,
            contents_updated,
        }
    }

    pub fn update(&mut self, focus: WindowFocus, index: usize) -> bool {
        let mouse_position = Vec2::from(input::mouse_position());

        let is_clicked = self.is_point_within_bounds(mouse_position)
            && input::is_mouse_button_pressed(MouseButton::Left);

        // Update grabbing
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
            && focus.grab.is_none()
            && input::is_mouse_button_down(MouseButton::Left)
            && self.is_point_within_title_bar(mouse_position)
        {
            self.grab_position = Some(mouse_position - self.position);
        } else {
            self.position =
                Self::position_from_proportionally(self.proportional_position, self.size);
        }

        self.update_scroll_bar(focus, index);

        self.text_offset = (self.scroll.floor() - self.scroll) * Self::TEXT_SIZE;

        is_clicked
    }

    pub fn update_scroll_bar(&mut self, focus: WindowFocus, index: usize) {
        let mouse_position = Vec2::from(input::mouse_position());

        if let Some(mut scroll_bar) = self.scroll_bar {
            let previous_vertical_offset = scroll_bar.vertical_offset;

            if scroll_bar.is_selected {
                let mouse_offset =
                    (mouse_position.y - self.position.y - Self::TITLE_HEIGHT) / scaling_factor();

                if let Some(grab_position) = scroll_bar.grab_position {
                    if input::is_mouse_button_down(MouseButton::Left) {
                        scroll_bar.vertical_offset = (mouse_offset - grab_position)
                            .clamp(0.0, self.height_of_editor() - scroll_bar.size.y);
                    } else {
                        scroll_bar.grab_position = None;
                    }
                } else if input::is_mouse_button_pressed(MouseButton::Left) {
                    if mouse_offset < scroll_bar.vertical_offset
                        || mouse_offset > scroll_bar.vertical_offset + scroll_bar.size.y
                    {
                        scroll_bar.vertical_offset = (mouse_offset - scroll_bar.size.y / 2.0)
                            .clamp(0.0, self.height_of_editor() - scroll_bar.size.y);
                    }

                    scroll_bar.grab_position = Some(mouse_offset - scroll_bar.vertical_offset);
                }
            }

            if scroll_bar.vertical_offset != previous_vertical_offset {
                self.scroll = scroll_bar.vertical_offset
                    / (self.height_of_editor() - scroll_bar.size.y)
                    * self.maximum_scroll();

                self.contents_updated = true;
            }

            self.scroll_bar = Some(scroll_bar);
        }

        let (scroll_bar_width, is_selected, grab_position) =
            if let Some(scroll_bar) = self.scroll_bar {
                let is_selected = scroll_bar.grab_position.is_some()
                    || focus.mouse == Some(index)
                        && self.is_point_within_scroll_bar_region(mouse_position);

                let target_width = if is_selected {
                    ScrollBar::SELECTED_WIDTH
                } else {
                    ScrollBar::UNSELECTED_WIDTH
                };

                let next_width = if (target_width - scroll_bar.size.x).abs() < 0.05 {
                    target_width
                } else {
                    let frame_time = macroquad::time::get_frame_time();

                    exp_decay(scroll_bar.size.x, target_width, 25.0, frame_time)
                };

                self.contents_updated |= next_width != scroll_bar.size.x;

                (next_width, is_selected, scroll_bar.grab_position)
            } else {
                (ScrollBar::UNSELECTED_WIDTH, false, None)
            };
        let scroll_bar_height = self.height_of_editor()
            / (self.text_editor.num_lines() as f32 * Self::TEXT_SIZE + self.height_of_editor()
                - Self::TEXT_SIZE);

        self.scroll_bar = (scroll_bar_height < 1.0).then(|| {
            let scroll_bar_height = (scroll_bar_height * self.height_of_editor()).max(40.0);

            let vertical_offset = (self.height_of_editor() - scroll_bar_height)
                * (self.scroll / self.maximum_scroll());

            ScrollBar {
                size: Vec2::new(scroll_bar_width, scroll_bar_height),
                vertical_offset,
                is_selected,
                grab_position,
            }
        });
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
    pub fn height_of_editor(&self) -> f32 {
        self.size.y - Self::TITLE_HEIGHT - Self::BORDER_WIDTH
    }

    #[must_use]
    pub fn maximum_scroll(&self) -> f32 {
        (self.text_editor.num_lines() - 1) as f32
    }

    #[must_use]
    pub fn is_grabbed(&self) -> bool {
        self.grab_position.is_some()
            || matches!(
                self.scroll_bar,
                Some(ScrollBar {
                    grab_position: Some(_),
                    ..
                })
            )
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
            Self::WINDOW_COLOR,
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
        if let Some(scroll_bar) = self.scroll_bar {
            shapes::draw_rectangle(
                self.size.x - scroll_bar.size.x,
                Self::TITLE_HEIGHT,
                scroll_bar.size.x - Self::BORDER_WIDTH,
                self.height_of_editor(),
                Self::WINDOW_COLOR,
            );

            shapes::draw_rectangle(
                self.size.x - scroll_bar.size.x,
                Self::TITLE_HEIGHT + scroll_bar.vertical_offset,
                scroll_bar.size.x,
                scroll_bar.size.y,
                ScrollBar::COLOR,
            );
        }

        camera::pop_camera_state();
    }

    pub fn text_params_with_size(text_size: f32) -> TextParams<'static> {
        TextParams {
            font: Some(&FONT),
            font_size: (text_size * Self::RESOLUTION_UPSCALING as f32) as u16,
            font_scale: 1.0 / Self::RESOLUTION_UPSCALING as f32,
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

    #[must_use]
    pub fn is_point_within_bounds(&self, point: Vec2) -> bool {
        point.x >= self.position.x - Self::WINDOW_PADDING * scaling_factor()
            && point.y >= self.position.y - Self::WINDOW_PADDING * scaling_factor()
            && point.x <= self.position.x + (self.size.x + Self::WINDOW_PADDING) * scaling_factor()
            && point.y <= self.position.y + (self.size.y + Self::WINDOW_PADDING) * scaling_factor()
    }

    #[must_use]
    pub fn is_point_within_title_bar(&self, point: Vec2) -> bool {
        point.x >= self.position.x - Self::WINDOW_PADDING * scaling_factor()
            && point.y >= self.position.y - Self::WINDOW_PADDING * scaling_factor()
            && point.x <= self.position.x + (self.size.x + Self::WINDOW_PADDING) * scaling_factor()
            && point.y <= self.position.y + Self::TITLE_HEIGHT * scaling_factor()
    }

    #[must_use]
    pub fn is_point_within_editor(&self, point: Vec2) -> bool {
        point.x >= self.position.x - Self::WINDOW_PADDING * scaling_factor()
            && point.y >= self.position.y + Self::TITLE_HEIGHT * scaling_factor()
            && point.x
                <= self.position.x
                    + (self.size.x
                        + if self.scroll_bar.is_some() {
                            -(ScrollBar::MAX_WIDTH + Self::ELEMENT_PADDING)
                        } else {
                            Self::WINDOW_PADDING
                        })
                        * scaling_factor()
            && point.y <= self.position.y + (self.size.y + Self::WINDOW_PADDING) * scaling_factor()
    }

    #[must_use]
    pub fn is_point_within_scroll_bar(&self, point: Vec2) -> bool {
        self.scroll_bar.is_some_and(|scroll_bar| {
            point.x
                >= self.position.x
                    + (self.size.x - ScrollBar::MAX_WIDTH - Self::ELEMENT_PADDING)
                        * scaling_factor()
                && point.y
                    >= self.position.y
                        + (Self::TITLE_HEIGHT + scroll_bar.vertical_offset) * scaling_factor()
                && point.x
                    <= self.position.x + (self.size.x + Self::ELEMENT_PADDING) * scaling_factor()
                && point.y
                    <= self.position.y
                        + (Self::TITLE_HEIGHT + scroll_bar.vertical_offset + scroll_bar.size.y)
                            * scaling_factor()
        })
    }

    #[must_use]
    pub fn is_point_within_scroll_bar_region(&self, point: Vec2) -> bool {
        self.scroll_bar.is_some()
            && point.x
                >= self.position.x
                    + (self.size.x - ScrollBar::MAX_WIDTH - Self::ELEMENT_PADDING)
                        * scaling_factor()
            && point.y >= self.position.y + Self::TITLE_HEIGHT * scaling_factor()
            && point.x <= self.position.x + (self.size.x + Self::ELEMENT_PADDING) * scaling_factor()
            && point.y <= self.position.y + (self.size.y - Self::BORDER_WIDTH) * scaling_factor()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollBar {
    pub size: Vec2,
    pub vertical_offset: f32,
    pub is_selected: bool,
    pub grab_position: Option<f32>,
}

impl ScrollBar {
    pub const COLOR: Color = colors::WHITE;

    pub const SELECTED_WIDTH: f32 = 7.5;
    pub const UNSELECTED_WIDTH: f32 = EditorWindow::BORDER_WIDTH;
    pub const MAX_WIDTH: f32 = Self::SELECTED_WIDTH.max(Self::UNSELECTED_WIDTH);
    pub const MIN_WIDTH: f32 = Self::SELECTED_WIDTH.min(Self::UNSELECTED_WIDTH);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowFocus {
    pub grab: Option<usize>,
    pub mouse: Option<usize>,
}

/// CREDIT: Freya Holmér: https://www.youtube.com/watch?v=LSNQuFEDOyQ
pub fn exp_decay(a: f32, b: f32, decay: f32, dt: f32) -> f32 {
    b + (a - b) * (-decay * dt).exp()
}
