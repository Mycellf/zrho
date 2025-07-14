use macroquad::{
    camera::{self, Camera2D},
    color::{Color, colors},
    input::{self, KeyCode, MouseButton},
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
    pub scaled_position: Vec2,

    pub size: Vec2,
    pub title: String,
    pub title_color: Color,

    pub grab_position: Option<Vec2>,
    pub is_focused: bool,
    pub key_repeats: KeyRepeats,

    pub scroll: f32,
    pub target_scroll: f32,
    pub scroll_bar: Option<ScrollBar>,
    pub text_offset: f32,

    pub text_editor: TextEditor,
    pub program: Result<Program, Vec<ProgramAssemblyError>>,
    pub target_computer: Computer,

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
        target_computer: Computer,
    ) -> EditorWindow {
        let position = Self::position_from_scaled(proportional_position, size);

        let grab_position = None;
        let is_focused = false;
        let key_repeats = KeyRepeats::default();

        let scroll = 0.0;
        let target_scroll = 0.0;
        let scroll_bar = None;
        let text_offset = 0.0;

        let program = Program::assemble_from(title.clone(), &text_editor.text, &target_computer);

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
            scaled_position: proportional_position,

            size,
            title,
            title_color,

            grab_position,
            is_focused,
            key_repeats,

            scroll,
            target_scroll,
            scroll_bar,
            text_offset,

            text_editor,
            program,
            target_computer,

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

                self.scaled_position = Self::scaled_position_from(self.position);
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
            self.position = Self::position_from_scaled(self.scaled_position, self.size);
        }

        self.update_editor();

        // Update scrolling
        let previous_scroll = self.scroll;

        if focus.mouse == Some(index) && !self.is_grabbed() {
            let scroll_input = input::mouse_wheel().1.clamp(-1.0, 1.0);

            if scroll_input != 0.0 {
                self.target_scroll -= scroll_input;

                if scroll_input.abs() >= 1.0 {
                    self.target_scroll = self.target_scroll.round();
                }
                self.target_scroll = self.target_scroll.clamp(0.0, self.maximum_scroll());
            }
        }

        if self.target_scroll != self.scroll {
            let frame_time = macroquad::time::get_frame_time();

            self.scroll =
                exp_decay_cutoff(self.scroll, self.target_scroll, 10.0, frame_time, 0.01).0;
        }

        self.contents_updated |= self.scroll != previous_scroll;

        self.update_scroll_bar(focus, index);

        self.text_offset = (self.scroll.floor() - self.scroll) * Self::TEXT_SIZE;

        is_clicked
    }

    pub fn update_editor(&mut self) {
        self.key_repeats.update();

        if self.is_grabbed() || !self.is_focused {
            return;
        }

        let mut moved_any_cursor = false;

        for i in 0..self.text_editor.cursors.len() {
            let mut cursor = self.text_editor.cursors[i];

            let mut moved = false;

            if self.is_key_pressed(KeyCode::Left) {
                cursor.position = self
                    .text_editor
                    .move_position_left(cursor.position, 1, true);
                moved = true;

                self.key_repeats.set_key(KeyCode::Left);
            }

            if self.is_key_pressed(KeyCode::Right) {
                cursor.position = self
                    .text_editor
                    .move_position_right(cursor.position, 1, true);
                moved = true;

                self.key_repeats.set_key(KeyCode::Right);
            }

            if self.is_key_pressed(KeyCode::Up) && cursor.position.line > 0 {
                cursor.position.line -= 1;
                moved = true;

                self.key_repeats.set_key(KeyCode::Up);
            }

            if self.is_key_pressed(KeyCode::Down)
                && cursor.position.line < self.text_editor.num_lines() - 1
            {
                cursor.position.line += 1;
                moved = true;

                self.key_repeats.set_key(KeyCode::Down);
            }

            if self.is_key_pressed(KeyCode::Home) {
                cursor.position.column = 0;
                moved = true;

                self.key_repeats.set_key(KeyCode::Home);
            }

            if self.is_key_pressed(KeyCode::End) {
                cursor.position.column = self
                    .text_editor
                    .length_of_line(cursor.position.line)
                    .unwrap();
                moved = true;

                self.key_repeats.set_key(KeyCode::End);
            }

            if self.is_key_pressed(KeyCode::PageUp) {
                cursor.position.line = (cursor.position.line)
                    .saturating_sub(self.height_of_editor_lines().saturating_sub(1));
                moved = true;

                self.key_repeats.set_key(KeyCode::PageUp);
            }

            if self.is_key_pressed(KeyCode::PageDown) {
                cursor.position.line = (cursor.position.line
                    + self.height_of_editor_lines().saturating_sub(1))
                .min(self.text_editor.num_lines() - 1);
                moved = true;

                self.key_repeats.set_key(KeyCode::PageDown);
            }

            if moved {
                cursor.index = self.text_editor.index_of_position(cursor.position).unwrap();

                self.text_editor.cursors[i] = cursor;
            }

            moved_any_cursor |= moved;
        }

        let mut typed = false;

        while let Some(mut character) = input::get_char_pressed() {
            if character == '\r' {
                character = '\n';
            }

            for i in 0..self.text_editor.cursors.len() {
                let cursor = self.text_editor.cursors[i];

                match character {
                    '\u{8}' if cursor.index > 0 => {
                        // Backspace
                        let range = self
                            .text_editor
                            .move_position_left(cursor.position, 1, true)
                            ..cursor.position;

                        self.text_editor.remove(range).unwrap();

                        typed = true;
                    }
                    // NOTE: The last character is always a newline, which has a length of 1
                    '\u{7f}' if cursor.index < self.text_editor.text.len() - 1 => {
                        // Delete
                        let range = cursor.position
                            ..self
                                .text_editor
                                .move_position_right(cursor.position, 1, true);

                        self.text_editor.remove(range).unwrap();

                        typed = true;
                    }
                    _ if !character.is_control() || character == '\n' => {
                        // Typed character
                        self.text_editor
                            .insert(cursor.position, &character.to_string())
                            .unwrap();

                        typed = true;
                    }
                    _ => (),
                }
            }
        }

        if moved_any_cursor || typed {
            for cursor in &self.text_editor.cursors {
                let position = cursor.position.line as f32;

                if position + 1.0 - self.height_of_editor() / Self::TEXT_SIZE > self.target_scroll {
                    self.target_scroll = position + 1.0 - self.height_of_editor() / Self::TEXT_SIZE;
                } else if position < self.target_scroll {
                    self.target_scroll = position;
                }
            }

            self.contents_updated = true;
        }

        if typed {
            self.program = Program::assemble_from(
                self.title.clone(),
                &self.text_editor.text,
                &self.target_computer,
            );
        }
    }

    pub fn update_scroll_bar(&mut self, focus: WindowFocus, index: usize) {
        let mouse_position = Vec2::from(input::mouse_position());

        if let Some(mut scroll_bar) = self.scroll_bar {
            let previous_vertical_offset = scroll_bar.vertical_offset;

            if scroll_bar.is_selected {
                let mouse_offset =
                    (mouse_position.y - self.position.y) / scaling_factor() - Self::TITLE_HEIGHT;

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

                self.target_scroll = self.scroll;

                self.contents_updated = true;
            }

            self.scroll_bar = Some(scroll_bar);
        }

        let (width, color, is_selected, grab_position) = if let Some(scroll_bar) = self.scroll_bar {
            let is_area_hovered = focus.mouse == Some(index)
                && self.is_point_within_scroll_bar_region(mouse_position);

            let is_hovered = is_area_hovered && self.is_point_within_scroll_bar(mouse_position);

            let is_grabbed = scroll_bar.grab_position.is_some();

            let frame_time = macroquad::time::get_frame_time();

            let target_width = if is_grabbed || is_area_hovered {
                ScrollBar::SELECTED_WIDTH
            } else {
                ScrollBar::UNSELECTED_WIDTH
            };

            let next_width =
                exp_decay_cutoff(scroll_bar.size.x, target_width, 25.0, frame_time, 0.05).0;

            let target_color = (is_area_hovered && !is_hovered && !is_grabbed) as isize as f32;

            let next_color =
                exp_decay_cutoff(scroll_bar.color, target_color, 50.0, frame_time, 0.05).0;

            self.contents_updated |=
                next_width != scroll_bar.size.x || next_color != scroll_bar.color;

            (
                next_width,
                next_color,
                is_grabbed || is_area_hovered,
                scroll_bar.grab_position,
            )
        } else {
            (ScrollBar::UNSELECTED_WIDTH, 0.0, false, None)
        };
        let height = self.height_of_editor()
            / (self.text_editor.num_lines() as f32 * Self::TEXT_SIZE + self.height_of_editor()
                - Self::TEXT_SIZE);

        self.scroll_bar = (height < 1.0).then(|| {
            let height = (height * self.height_of_editor()).max(40.0);

            let vertical_offset =
                (self.height_of_editor() - height) * (self.scroll / self.maximum_scroll());

            ScrollBar {
                size: Vec2::new(width, height),
                vertical_offset,
                is_selected,
                grab_position,
                color,
            }
        });
    }

    pub fn is_key_pressed(&mut self, key_code: KeyCode) -> bool {
        self.key_repeats.key == Some(key_code) || input::is_key_pressed(key_code)
    }

    pub fn clamp_within_window_boundary(&mut self) {
        self.position = Self::clamp_position_within_window_boundary(self.position, self.size);
    }

    #[must_use]
    pub fn height_of_editor(&self) -> f32 {
        self.size.y - Self::TITLE_HEIGHT - Self::BORDER_WIDTH
    }

    #[must_use]
    pub fn height_of_editor_lines(&self) -> usize {
        (self.height_of_editor() / Self::TEXT_SIZE).floor() as usize
    }

    #[must_use]
    pub fn maximum_scroll(&self) -> f32 {
        (self.text_editor.num_lines() - 1) as f32
    }

    #[must_use]
    pub fn should_hold_mouse_focus(&self) -> bool {
        self.is_grabbed()
    }

    #[must_use]
    pub fn is_grabbed(&self) -> bool {
        self.is_being_dragged() || self.is_scroll_bar_grabbed()
    }

    #[must_use]
    pub fn is_being_dragged(&self) -> bool {
        self.grab_position.is_some()
    }

    #[must_use]
    pub fn is_scroll_bar_grabbed(&self) -> bool {
        matches!(
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
        let mut start_line = self.scroll.floor() as usize;
        let end_line = (self.scroll + self.height_of_editor() / Self::TEXT_SIZE).ceil() as usize;

        let mut text_offset = self.text_offset;

        if start_line > 0 {
            start_line -= 1;
            text_offset -= Self::TEXT_SIZE;
        }

        self.text_editor.draw_range(
            start_line..end_line,
            Vec2::new(Self::BORDER_WIDTH + 5.0, Self::TITLE_HEIGHT + text_offset),
            Self::TEXT_SIZE,
            1.0,
            1.0,
        );

        // Cursors
        let TextParams {
            font_size,
            font_scale,
            ..
        } = Self::text_params_with_size(Self::TEXT_SIZE);

        for cursor in &self.text_editor.cursors {
            if !(start_line..end_line).contains(&cursor.position.line) {
                continue;
            }

            let line_start_index = self.text_editor.lines[cursor.position.line].byte_offset;
            let preceding_contents = &self.text_editor.text[line_start_index..cursor.index];

            shapes::draw_rectangle(
                text::measure_text(preceding_contents, Some(&FONT), font_size, font_scale).width
                    + Self::BORDER_WIDTH
                    + 5.0,
                (cursor.position.line as f32 - self.scroll) * Self::TEXT_SIZE + Self::TITLE_HEIGHT,
                1.0,
                Self::TEXT_SIZE,
                colors::WHITE,
            );
        }

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
                scroll_bar.color(),
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
    pub fn scaled_position_from(position: Vec2) -> Vec2 {
        position / scaling_factor()
    }

    #[must_use]
    pub fn position_from_scaled(proportional_position: Vec2, size: Vec2) -> Vec2 {
        Self::clamp_position_within_window_boundary(proportional_position * scaling_factor(), size)
    }

    #[must_use]
    pub fn clamp_position_within_window_boundary(mut position: Vec2, size: Vec2) -> Vec2 {
        let scaling_factor = scaling_factor();

        if position.x + size.x * scaling_factor > window::screen_width() {
            position.x = window::screen_width() - size.x * scaling_factor;
        }

        if position.x < 0.0 {
            position.x = 0.0;
        }

        if position.y + size.y * scaling_factor > window::screen_height() {
            position.y = window::screen_height() - size.y * scaling_factor;
        }

        if position.y < 0.0 {
            position.y = 0.0;
        }

        position
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
    pub color: f32,
}

impl ScrollBar {
    pub const BASE_COLOR: Color = colors::WHITE;
    pub const ALTERNATE_COLOR: Color = colors::LIGHTGRAY;

    pub const SELECTED_WIDTH: f32 = 7.5;
    pub const UNSELECTED_WIDTH: f32 = EditorWindow::BORDER_WIDTH;
    pub const MAX_WIDTH: f32 = Self::SELECTED_WIDTH.max(Self::UNSELECTED_WIDTH);
    pub const MIN_WIDTH: f32 = Self::SELECTED_WIDTH.min(Self::UNSELECTED_WIDTH);

    pub fn color(&self) -> Color {
        color_lerp(Self::BASE_COLOR, Self::ALTERNATE_COLOR, self.color)
    }
}

/// HACK: This exists because macroquad won't give key repeats for the navigation keys
#[derive(Clone, Copy, Debug)]
pub struct KeyRepeats {
    pub delay: f32,
    pub interval: f32,
    pub state: Option<(KeyCode, f32)>,
    pub key: Option<KeyCode>,
}

impl KeyRepeats {
    pub fn update(&mut self) {
        self.key = if let &mut Some((key_code, ref mut time)) = &mut self.state {
            if input::is_key_down(key_code) {
                *time -= macroquad::time::get_frame_time();

                (*time <= 0.0).then(|| {
                    *time = self.interval;

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
        Self {
            delay: 0.5,
            interval: 0.03,
            state: None,
            key: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowFocus {
    pub grab: Option<usize>,
    pub mouse: Option<usize>,
}

pub fn exp_decay_cutoff(a: f32, b: f32, decay: f32, dt: f32, cutoff: f32) -> (f32, bool) {
    if (a - b).abs() < cutoff {
        (b, true)
    } else {
        (exp_decay(a, b, decay, dt), false)
    }
}

/// CREDIT: Freya Holmér: https://www.youtube.com/watch?v=LSNQuFEDOyQ
pub fn exp_decay(a: f32, b: f32, decay: f32, dt: f32) -> f32 {
    b + (a - b) * (-decay * dt).exp()
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn color_lerp(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: lerp(a.r, b.r, t),
        g: lerp(a.g, b.g, t),
        b: lerp(a.b, b.b, t),
        a: lerp(a.a, b.a, t),
    }
}
