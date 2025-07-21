use macroquad::{
    camera::{self, Camera2D},
    color::{Color, colors},
    input::{self, KeyCode, MouseButton},
    math::{Rect, Vec2},
    text::{self, TextParams},
    texture::{self, DrawTextureParams},
};

use super::{
    FONT,
    element::{DrawArea, Element, UpdateContext, UpdateResult, WindowFocusUse},
    scroll_bar::ScrollableElement,
    text_editor_operations::{CharacterPosition, Cursor, CursorLocation, TextEditorOperations},
    window::{self, Window},
};

#[derive(Debug)]
pub struct TextEditor {
    pub editor: TextEditorOperations,
    pub border_size: f32,
    pub lines: f32,

    pub scroll: f32,
    pub target_scroll: f32,
    pub scroll_speed: f32,

    pub camera: Camera2D,
    pub text_needs_update: bool,
    pub page_size: Vec2,
    pub cursors_fit_in_page: bool,
}

impl Element for TextEditor {
    fn height(&self) -> f32 {
        self.lines * Self::TEXT_HEIGHT + self.border_size * 2.0
    }

    fn uses_keyboard_focus(&self) -> bool {
        true
    }

    fn uses_window_focus(&self) -> WindowFocusUse {
        WindowFocusUse::WithKeyboardFocus
    }

    fn update(&mut self, context: UpdateContext) -> UpdateResult {
        let UpdateContext { focus, .. } = context;
        let mut result = UpdateResult::default();

        // Scrolling
        let previous_scroll = self.scroll;

        if focus.mouse_hover {
            let scroll_input = input::mouse_wheel().1.clamp(-1.0, 1.0);

            if scroll_input != 0.0 {
                self.target_scroll -= scroll_input;

                if scroll_input.abs() >= 1.0 {
                    self.target_scroll = self.target_scroll.round();
                }
                self.target_scroll = self
                    .target_scroll
                    .clamp(0.0, self.maximum_scroll() / Self::TEXT_HEIGHT);
                self.scroll_speed = Self::SCROLL_SPEED;
            }
        }

        if self.target_scroll != self.scroll {
            let frame_time = macroquad::time::get_frame_time();

            self.scroll = super::exp_decay_cutoff(
                self.scroll,
                self.target_scroll,
                self.scroll_speed,
                frame_time,
                0.01,
            )
            .0;
        }

        result.update_graphics |= self.scroll != previous_scroll;
        self.text_needs_update |= self.scroll.max(1.0).floor() != previous_scroll.max(1.0).floor();

        // Editor controls
        if focus.mouse_hover || focus.mouse_grab {
            result |= self.move_cursors_with_mouse(context);
        }

        if focus.keyboard {
            let (mut moved, mut page_scrolled) =
                (self.editor).move_cursors_with_keybinds(self.lines.floor() as usize);

            if moved {
                self.editor.history.finish_edit_group();
            }

            let (moved_by_typing, page_scrolled_by_typing, typed, seperate_edits_in_history) =
                self.editor.type_from_input_characters();

            moved |= moved_by_typing;
            self.text_needs_update |= typed;

            page_scrolled |= page_scrolled_by_typing;

            if seperate_edits_in_history {
                self.editor.history.finish_edit_group();
            }

            self.editor.history.insert_buffered_edits();

            if seperate_edits_in_history {
                self.editor.history.finish_edit_group();
            }

            if moved || typed {
                result.update_graphics = true;

                self.editor.deduplicate_cursors();
            }

            if moved {
                self.scroll_to_cursors(
                    page_scrolled,
                    self.editor
                        .cursors
                        .clone()
                        .into_iter()
                        .map(|cursor| cursor.position.line),
                );
            }

            // TODO:
            //
            // if typed {
            //     self.target_computer.reset();
            //     self.highlighted_lines = Vec::new();
            //     self.program_active = false;
            //
            //     self.program = Program::assemble_from(
            //         self.title.clone(),
            //         &self.editor.text,
            //         &self.target_computer,
            //     );
            // }
        }

        result
    }

    fn draw(&mut self, context: UpdateContext) {
        let UpdateContext { window, area, .. } = context;

        area.draw_rectangle(window.theme.darker_background_color);

        self.draw_selections(context);
        self.draw_text(context);
        self.draw_cursors(context);

        if self.border_size > 0.0 {
            area.draw_rectangle_lines(self.border_size * 2.0, window.theme.background_color);
        }
    }

    fn force_update(&mut self) {
        self.text_needs_update = true;
    }
}

impl ScrollableElement for TextEditor {
    fn set_scroll(&mut self, scroll: f32) {
        let previous_scroll = self.scroll;

        self.target_scroll = scroll.clamp(0.0, self.maximum_scroll()) / Self::TEXT_HEIGHT;
        self.scroll = self.target_scroll;

        self.text_needs_update |= self.scroll.max(1.0).floor() != previous_scroll.max(1.0).floor();
    }

    fn get_scroll(&self) -> f32 {
        self.scroll * Self::TEXT_HEIGHT
    }

    fn maximum_scroll(&self) -> f32 {
        (self.editor.num_lines() - 1) as f32 * Self::TEXT_HEIGHT
    }

    fn page_height(&self) -> f32 {
        self.lines * Self::TEXT_HEIGHT
    }

    fn page_offset(&self) -> f32 {
        self.border_size
    }
}

impl TextEditor {
    pub const TEXT_OFFSET: Vec2 = Vec2::new(5.0, 0.0);
    pub const TEXT_HEIGHT: f32 = Window::BASE_TEXT_HEIGHT;
    pub const TEXT_WIDTH: f32 = Window::BASE_TEXT_WIDTH;

    pub const CURSOR_COLOR_FOCUSED: Color = colors::WHITE;
    pub const CURSOR_COLOR_UNFOCUSED: Color = colors::GRAY;

    pub const SCROLL_SPEED: f32 = 25.0;
    pub const FOLLOW_SPEED: f32 = f32::INFINITY;
    pub const PAGE_FOLLOW_SPEED: f32 = 10.0;

    pub const SELECTION_COLOR: Color = Color {
        a: 2.0 / 7.0,
        ..Color::from_hex(0x60a0ff)
    };

    #[must_use]
    pub fn new(editor: TextEditorOperations, lines: f32, border_size: f32) -> Self {
        Self {
            editor,
            border_size,
            lines,

            scroll: 0.0,
            target_scroll: 0.0,
            scroll_speed: 0.0,

            camera: Camera2D {
                offset: Vec2::new(-1.0, -1.0),
                ..Default::default()
            },
            text_needs_update: false,
            page_size: Vec2::ZERO,
            cursors_fit_in_page: true,
        }
    }

    #[must_use]
    pub fn text_offset(&self) -> Vec2 {
        Self::TEXT_OFFSET + self.border_size
    }

    #[must_use]
    pub fn page_size(&self, area: DrawArea) -> Vec2 {
        area.size - self.border_size * 2.0
    }

    pub fn draw_text(&mut self, UpdateContext { area, .. }: UpdateContext) {
        let page_size = self.page_size(area);
        let update_page_texture = page_size != self.page_size;

        if update_page_texture {
            self.text_needs_update = true;

            self.page_size = page_size;

            let target_logical_size = self.page_size + Vec2::new(0.0, Self::TEXT_HEIGHT * 2.0);

            let target_size = target_logical_size * Window::RESOLUTION_UPSCALING;

            self.camera.render_target = Some(texture::render_target(
                target_size.x as u32,
                target_size.y as u32,
            ));

            self.camera.zoom = 2.0 / target_logical_size;
        }

        if self.text_needs_update {
            self.text_needs_update = false;

            camera::push_camera_state();
            camera::set_camera(&self.camera);

            macroquad::window::clear_background(colors::BLANK);

            let start_line = self.scroll.max(1.0).floor() as usize - 1;
            let end_line = start_line + self.lines.ceil() as usize + 2;

            self.editor.draw_range(
                start_line..end_line,
                colors::BLANK,
                &[],
                Self::TEXT_OFFSET,
                Self::TEXT_HEIGHT,
                1.0,
                1.0,
            );

            camera::pop_camera_state();
        }

        let offset = area.offset + self.border_size;
        let scroll = if self.scroll < 1.0 {
            self.scroll
        } else {
            1.0 + self.scroll % 1.0
        };

        texture::draw_texture_ex(
            &self.camera.render_target.as_ref().unwrap().texture,
            offset.x,
            offset.y,
            colors::WHITE,
            DrawTextureParams {
                dest_size: Some(self.page_size),
                source: Some(Rect {
                    x: 0.0,
                    y: scroll * Self::TEXT_HEIGHT * Window::RESOLUTION_UPSCALING,
                    w: self.page_size.x * Window::RESOLUTION_UPSCALING,
                    h: self.page_size.y * Window::RESOLUTION_UPSCALING,
                }),
                ..Default::default()
            },
        );
    }

    pub fn draw_cursors(&self, UpdateContext { area, focus, .. }: UpdateContext) {
        let TextParams {
            font_size,
            font_scale,
            ..
        } = window::text_params_with_height(Self::TEXT_HEIGHT);

        let start_line = self.scroll.floor() as usize;
        let end_line = (self.scroll + self.lines).ceil() as usize;

        for cursor in &self.editor.cursors {
            if cursor.position.line < start_line || cursor.position.line >= end_line {
                continue;
            }

            let line_start_index = self.editor.lines[cursor.position.line].byte_offset;
            let preceding_contents = &self.editor.text[line_start_index..cursor.index];

            area.draw_rectangle_inside(
                area.offset
                    + self.text_offset()
                    + Vec2::new(
                        text::measure_text(preceding_contents, Some(&FONT), font_size, font_scale)
                            .width,
                        self.vertical_offset_of_line(cursor.position.line),
                    ),
                Vec2::new(1.0, Self::TEXT_HEIGHT),
                if focus.keyboard {
                    Self::CURSOR_COLOR_FOCUSED
                } else {
                    Self::CURSOR_COLOR_UNFOCUSED
                },
            );
        }
    }

    pub fn draw_selections(&self, UpdateContext { area, .. }: UpdateContext) {
        let TextParams {
            font_size,
            font_scale,
            ..
        } = window::text_params_with_height(Self::TEXT_HEIGHT);

        let start_line = self.scroll.floor() as usize;
        let end_line = (self.scroll + self.lines).ceil() as usize;

        for cursor in &self.editor.cursors {
            let Some(end) = cursor.end else {
                continue;
            };

            let start = cursor.start;

            let (start, end) = if start.index > end.index {
                (end, start)
            } else {
                (start, end)
            };

            let starts_before = start.position.line < start_line;
            let ends_before = end.position.line < start_line;

            let starts_after = start.position.line >= end_line;
            let ends_after = end.position.line >= end_line;

            if starts_before && ends_before || starts_after && ends_after {
                continue;
            }

            let start_index = self.editor.lines[start.position.line].byte_offset;
            let start_contents = &self.editor.text[start_index..start.index];

            let end_index = self.editor.lines[end.position.line].byte_offset;
            let end_contents = &self.editor.text[end_index..end.index];

            let start_offset =
                text::measure_text(start_contents, Some(&FONT), font_size, font_scale).width;
            let end_offset =
                text::measure_text(end_contents, Some(&FONT), font_size, font_scale).width;

            if start.position.line == end.position.line {
                area.draw_rectangle_inside(
                    area.offset
                        + self.text_offset()
                        + Vec2::new(
                            start_offset,
                            self.vertical_offset_of_line(start.position.line),
                        ),
                    Vec2::new(end_offset - start_offset, Self::TEXT_HEIGHT),
                    Self::SELECTION_COLOR,
                );
            } else {
                if start.position.line >= start_line {
                    area.draw_rectangle_inside(
                        area.offset
                            + self.text_offset()
                            + Vec2::new(
                                start_offset,
                                self.vertical_offset_of_line(start.position.line),
                            ),
                        Vec2::new(
                            self.width_of_line(start.position.line).unwrap() - start_offset
                                + Self::TEXT_WIDTH,
                            Self::TEXT_HEIGHT,
                        ),
                        Self::SELECTION_COLOR,
                    );
                }

                for line in start.position.line + 1..end.position.line {
                    area.draw_rectangle_inside(
                        area.offset
                            + self.text_offset()
                            + Vec2::new(0.0, self.vertical_offset_of_line(line)),
                        Vec2::new(
                            self.width_of_line(line).unwrap() + Self::TEXT_WIDTH,
                            Self::TEXT_HEIGHT,
                        ),
                        Self::SELECTION_COLOR,
                    );
                }

                if end.position.line <= end_line {
                    area.draw_rectangle_inside(
                        area.offset
                            + self.text_offset()
                            + Vec2::new(0.0, self.vertical_offset_of_line(end.position.line)),
                        Vec2::new(end_offset, Self::TEXT_HEIGHT),
                        Self::SELECTION_COLOR,
                    );
                }
            }
        }
    }

    #[must_use]
    pub fn vertical_offset_of_line(&self, line: usize) -> f32 {
        (line as f32 - self.scroll) * Self::TEXT_HEIGHT
    }

    #[must_use]
    pub fn width_of_line(&self, line: usize) -> Option<f32> {
        let contents = self.editor.get_line(line)?;

        let TextParams {
            font_size,
            font_scale,
            ..
        } = window::text_params_with_height(Self::TEXT_HEIGHT);

        Some(text::measure_text(contents, Some(&FONT), font_size, font_scale).width)
    }

    pub fn scroll_to_cursors(
        &mut self,
        page_scrolled: bool,
        lines: impl Iterator<Item = usize> + Clone,
    ) {
        if lines.clone().peekable().peek().is_none() {
            return;
        }

        let min_line = lines.clone().min().unwrap() as f32;

        let max_line = lines.max().unwrap() as f32;

        let height_offset = self.lines - 1.0;

        let cursors_fit_in_window = min_line >= max_line - height_offset;

        let (min_scroll, max_scroll) = if cursors_fit_in_window {
            (max_line - height_offset, min_line)
        } else {
            (min_line - height_offset, max_line)
        };

        let mut follow_scroll = false;

        if self.target_scroll > max_scroll {
            self.target_scroll = max_scroll;
            follow_scroll = true;
        } else if self.target_scroll < min_scroll {
            self.target_scroll = min_scroll;
            follow_scroll = true;
        }

        if follow_scroll {
            self.scroll_speed =
                if page_scrolled || cursors_fit_in_window && !self.cursors_fit_in_page {
                    Self::PAGE_FOLLOW_SPEED
                } else {
                    Self::FOLLOW_SPEED
                };
        }

        self.cursors_fit_in_page = cursors_fit_in_window;
    }

    #[must_use]
    pub fn position_of_point_in_text(
        &self,
        mut point: Vec2,
        clamp: bool,
        UpdateContext { window, area, .. }: UpdateContext,
    ) -> Option<CharacterPosition> {
        if !(clamp || area.contains_point(window.mouse_position())) {
            return None;
        }

        if clamp {
            point = area.clamp_point(point);
        }

        let point = point - area.offset;

        let text_window_space = point - self.text_offset();

        let text_space = text_window_space + Vec2::Y * self.scroll * Self::TEXT_HEIGHT;

        let column = (text_space.x / Self::TEXT_WIDTH).round().max(0.0) as usize;
        let line = (text_space.y / Self::TEXT_HEIGHT).max(0.0) as usize;

        let final_line = self.editor.num_lines() - 1;

        Some(if line > final_line {
            CharacterPosition {
                line: final_line,
                column: self.editor.length_of_line(final_line).unwrap(),
            }
        } else {
            CharacterPosition { line, column }
        })
    }

    #[must_use]
    pub fn move_cursors_with_mouse(&mut self, context: UpdateContext) -> UpdateResult {
        let UpdateContext { window, focus, .. } = context;

        let mut result = UpdateResult {
            grab_mouse: focus.mouse_grab,
            ..Default::default()
        };

        if !(result.grab_mouse
            || focus.mouse_hover && input::is_mouse_button_pressed(MouseButton::Left))
        {
            return result;
        }

        let mouse_position = window.mouse_position();

        if result.grab_mouse && !input::is_mouse_button_down(MouseButton::Left) {
            result.grab_mouse = false;
        } else if let Some(position) =
            self.position_of_point_in_text(mouse_position, result.grab_mouse, context)
        {
            let clicked_location = CursorLocation {
                position,
                index: self.editor.index_of_position(position).unwrap(),
            };

            let alt = input::is_key_down(KeyCode::LeftAlt) || input::is_key_down(KeyCode::RightAlt);

            if input::is_key_down(KeyCode::LeftShift)
                || input::is_key_down(KeyCode::RightShift)
                || result.grab_mouse
            {
                let cursor = self.editor.cursors.last_mut().unwrap();
                if cursor.end.is_none() {
                    cursor.end = Some(cursor.start);
                }

                result.update_graphics |= cursor.start != clicked_location;

                cursor.start = clicked_location;

                if let Some(end) = cursor.end {
                    if end == cursor.start {
                        cursor.end = None;
                    }
                }

                result.grab_mouse = true;
            } else {
                let cursor = Cursor {
                    start: clicked_location,
                    ..Default::default()
                };

                if alt {
                    if let Some(index) = (self.editor.cursors)
                        .iter()
                        .position(|&other_cursor| other_cursor == cursor)
                    {
                        if self.editor.cursors.len() > 1 {
                            self.editor.cursors.remove(index);
                        }
                    } else {
                        self.editor.cursors.push(cursor);
                        result.grab_mouse = true;
                    }
                } else {
                    self.editor.cursors = vec![cursor];
                    result.grab_mouse = true;
                }

                result.update_graphics = true;
            }
        }

        self.editor.history.finish_edit_group();

        result
    }
}
