use std::{
    cell::{LazyCell, OnceCell},
    ops::{Deref, DerefMut, Range},
    sync::Mutex,
};

use macroquad::{
    color::{Color, colors},
    input::{self, KeyCode},
    math::Vec2,
    text::{self, TextDimensions, TextParams},
};

use super::window;

#[derive(Clone, Debug)]
pub struct TextEditorOperations {
    pub text: String,
    pub lines: Vec<Line>,
    pub cursors: Vec<Cursor>,

    pub history: EditHistory,
}

impl TextEditorOperations {
    #[must_use]
    pub fn new(text: String) -> Self {
        let lines = Self::line_indecies_from(&text);
        let cursors = vec![Cursor::default()];

        let history = EditHistory::default();

        let mut result = Self {
            text,
            lines,
            cursors,

            history,
        };

        result.update_colors_of_all_lines();

        result
    }

    #[must_use]
    pub fn line_indecies_from(text: &str) -> Vec<Line> {
        let mut lines = Vec::new();

        lines.push(Line::from_byte_offset(0));

        for (i, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                lines.push(Line::from_byte_offset(i + 1));
            }
        }

        lines
    }

    #[must_use]
    pub fn num_lines(&self) -> usize {
        self.lines.len() - 1
    }

    pub fn update_colors_of_all_lines(&mut self) {
        for i in 0..self.lines.len() {
            self.update_colors_of_line(i).unwrap();
        }
    }

    #[must_use]
    pub fn update_colors_of_line(&mut self, index: usize) -> Option<()> {
        let range = self.byte_range_of_line(index)?;

        self.lines[index].update_colors_from(&self.text[range]);

        Some(())
    }

    pub fn draw_all(
        &self,
        background_color: Color,
        highlighted_lines: &[usize],
        position: Vec2,
        text_size: f32,
        line_height: f32,
        character_width: f32,
    ) {
        self.draw_range(
            0..self.lines.len(),
            background_color,
            highlighted_lines,
            position,
            text_size,
            line_height,
            character_width,
        );
    }

    pub fn draw_range(
        &self,
        lines: Range<usize>,
        background_color: Color,
        highlighted_lines: &[usize],
        mut position: Vec2,
        text_size: f32,
        line_height: f32,
        character_width: f32,
    ) {
        let line_height = line_height * text_size;

        for i in lines {
            let Some(segments) = self.color_segments_of_line(i) else {
                continue;
            };

            position.y += line_height;
            let mut line_position = position - Vec2::Y * line_height * 0.125;

            if highlighted_lines.contains(&i) {
                text::draw_text_ex(
                    self.get_line(i).unwrap(),
                    line_position.x,
                    line_position.y,
                    TextParams {
                        font_scale_aspect: character_width,
                        color: background_color,
                        ..window::text_params_with_height(text_size)
                    },
                );

                continue;
            }

            for (range, color_choice) in segments {
                let segment_text = &self.text[range];
                let segment_color = Color::from(color_choice);

                let TextDimensions { width, .. } = text::draw_text_ex(
                    segment_text,
                    line_position.x,
                    line_position.y,
                    TextParams {
                        font_scale_aspect: character_width,
                        color: segment_color,
                        ..window::text_params_with_height(text_size)
                    },
                );

                line_position.x += width;
            }
        }
    }

    #[must_use]
    pub fn insert(&mut self, position: CharacterPosition, text: &str) -> Option<()> {
        self.replace(position..position, text)
    }

    #[must_use]
    pub fn remove(&mut self, range: Range<CharacterPosition>) -> Option<()> {
        self.replace(range, "")
    }

    #[must_use]
    pub fn replace(&mut self, range: Range<CharacterPosition>, text: &str) -> Option<()> {
        let start_index = self.index_of_position(range.start)?;
        let end_index = self.index_of_position(range.end)?;

        let replaced = self.text[start_index..end_index].to_owned();

        self.replace_without_history(range, text)?;

        self.history.add_edit_to_buffer(Edit {
            start: start_index,
            inserted: text.to_owned(),
            replaced,
        });

        Some(())
    }

    #[must_use]
    fn replace_without_history(
        &mut self,
        range: Range<CharacterPosition>,
        text: &str,
    ) -> Option<()> {
        let start_index = self.index_of_position(range.start)?;
        let end_index = self.index_of_position(range.end)?;

        let removed_bytes = end_index.checked_sub(start_index)?;
        let removed_lines = range.end.line.checked_sub(range.start.line)?;

        for moved_line in &mut self.lines[range.end.line + 1..] {
            moved_line.byte_offset += text.len();
            moved_line.byte_offset -= removed_bytes;
        }

        let mut new_lines = Vec::new();

        for (i, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                new_lines.push(Line::from_byte_offset(start_index + i + 1));
            }
        }

        let num_new_lines = new_lines.len();

        self.lines
            .splice(range.start.line + 1..range.end.line + 1, new_lines);

        self.text.replace_range(start_index..end_index, text);

        for i in 0..self.cursors.len() {
            let update_location = |mut location: CursorLocation| {
                if location.index >= start_index {
                    if location.index < end_index {
                        location.index = start_index;
                    } else {
                        location.index -= removed_bytes;
                    }

                    location.index += text.len();

                    if location.position.line <= range.end.line {
                        let index = location.index;
                        location.position = self.position_of_index(index).unwrap();
                    } else {
                        location.position.line += num_new_lines;
                        location.position.line -= removed_lines;
                    }
                }

                location
            };

            let mut cursor = self.cursors[i];

            cursor.start = update_location(cursor.start);
            cursor.end = cursor.end.map(update_location);

            self.cursors[i] = cursor;
        }

        for line in range.start.line..range.start.line + num_new_lines + 1 {
            self.update_colors_of_line(line).unwrap();
        }

        Some(())
    }

    pub fn redo(&mut self) {
        if let Some(group) = self.history.redo() {
            for edit in group.edits {
                let start_index = edit.start;
                let end_index = edit.start + edit.replaced.len();

                let range = self.position_of_index(start_index).unwrap()
                    ..self.position_of_index(end_index).unwrap();

                self.replace_without_history(range, &edit.inserted).unwrap();
            }
        }
    }

    pub fn undo(&mut self) {
        if let Some(group) = self.history.undo() {
            for edit in group.edits.into_iter().rev() {
                let start_index = edit.start;
                let end_index = edit.start + edit.inserted.len();

                let range = self.position_of_index(start_index).unwrap()
                    ..self.position_of_index(end_index).unwrap();

                self.replace_without_history(range, &edit.replaced).unwrap();
            }
        }
    }

    pub fn deduplicate_cursors(&mut self) {
        let old_cursors = std::mem::take(&mut self.cursors);

        for cursor in old_cursors {
            if !self.cursors.contains(&cursor) {
                self.cursors.push(cursor);
            }
        }
    }

    #[must_use]
    pub fn move_position_left(
        &self,
        mut position: CharacterPosition,
        mut offset: usize,
        wrap: bool,
    ) -> CharacterPosition {
        while offset > 0 {
            if offset <= position.column || !wrap || position.line == 0 {
                position.column = position.column.saturating_sub(offset);
                break;
            }

            offset -= position.column + 1;
            position.line -= 1;
            position.column = self.length_of_line(position.line).unwrap();
        }

        position
    }

    #[must_use]
    pub fn move_position_right(
        &self,
        mut position: CharacterPosition,
        mut offset: usize,
        wrap: bool,
    ) -> CharacterPosition {
        while offset > 0 {
            let length = self.length_of_line(position.line).unwrap();

            if offset <= length - position.column || !wrap || position.line >= self.num_lines() - 1
            {
                position.column = (position.column + offset).min(length);
                break;
            }

            offset -= length - position.column + 1;
            position.line += 1;
            position.column = 0;
        }

        position
    }

    #[must_use]
    pub fn constrain_position_to_contents(&self, position: CharacterPosition) -> CharacterPosition {
        let final_line = self.num_lines() - 1;

        if position.line > final_line {
            CharacterPosition {
                line: final_line,
                column: self.length_of_line(final_line).unwrap(),
            }
        } else {
            CharacterPosition {
                line: position.line,
                column: (position.column).min(self.length_of_line(position.line).unwrap()),
            }
        }
    }

    #[must_use]
    pub fn length_of_line(&self, line: usize) -> Option<usize> {
        Some(self.get_line(line)?.chars().count())
    }

    #[must_use]
    pub fn columns_in_line(&self, index: usize) -> Option<usize> {
        Some(self.get_line(index)?.chars().count())
    }

    /// Does not include any newlines at the end of the line
    #[must_use]
    pub fn byte_range_of_line(&self, index: usize) -> Option<Range<usize>> {
        let start = self.lines.get(index)?;

        if let Some(end) = self.lines.get(index + 1) {
            Some(start.byte_offset..end.byte_offset - 1)
        } else {
            Some(start.byte_offset..self.text.len())
        }
    }

    #[must_use]
    pub fn color_segments_of_line(&self, index: usize) -> Option<Vec<(Range<usize>, ColorChoice)>> {
        let line = self.lines.get(index)?;

        let full_range = self.byte_range_of_line(index)?;

        let mut start = full_range.start;
        let mut color_choice = ColorChoice::default();

        let mut colors = Vec::new();

        for &LineSegmentColor {
            relative_offset,
            color_choice: next_color_choice,
        } in &line.colors
        {
            let end = full_range.start + relative_offset;

            colors.push((start..end, color_choice));

            color_choice = next_color_choice;
            start = end;
        }

        colors.push((start..full_range.end, color_choice));

        Some(colors)
    }

    #[must_use]
    pub fn index_of_position(&self, position: CharacterPosition) -> Option<usize> {
        let range = self.byte_range_of_line(position.line)?;
        let column_byte_offset = &self.text[range.clone()]
            .chars()
            .take(position.column)
            .map(char::len_utf8)
            .sum();

        Some(range.start + column_byte_offset)
    }

    #[must_use]
    pub fn position_of_index(&self, index: usize) -> Option<CharacterPosition> {
        if index > self.text.len() {
            return None;
        }

        let line = match (self.lines).binary_search_by_key(&index, |line| line.byte_offset) {
            Ok(line) => line,
            Err(line) => line - 1,
        };

        let column = self
            .get_line(line)
            .unwrap()
            .chars()
            .scan(self.lines[line].byte_offset, |acc, character| {
                if *acc >= index {
                    return None;
                }

                *acc += character.len_utf8();

                assert!(*acc <= index, "Index {index} is inside a codepoint");

                Some(())
            })
            .count();

        Some(CharacterPosition { line, column })
    }

    #[must_use]
    pub fn get_line(&self, index: usize) -> Option<&str> {
        let range = self.byte_range_of_line(index)?;

        Some(&self.text[range])
    }

    #[must_use]
    pub fn get_character(&self, position: CharacterPosition) -> Option<char> {
        self.get_line(position.line)?.chars().nth(position.column)
    }

    #[must_use]
    pub fn move_cursors_with_keybinds(&mut self, page_height: usize) -> (bool, bool) {
        let mut moved_any_cursor = false;
        let mut follow_slowly = false;

        for i in 0..self.cursors.len() {
            let mut cursor = self.cursors[i];

            let mut moved = false;

            if super::is_key_typed(KeyCode::Left) {
                cursor.position = self.constrain_position_to_contents(cursor.position);

                cursor.position = self.move_position_left(cursor.position, 1, true);
                moved = true;
            }

            if super::is_key_typed(KeyCode::Right) {
                cursor.position = self.constrain_position_to_contents(cursor.position);

                cursor.position = self.move_position_right(cursor.position, 1, true);
                moved = true;
            }

            if super::is_key_typed(KeyCode::Up) {
                if cursor.position.line > 0 {
                    cursor.position.line -= 1;
                } else {
                    cursor.position.column = 0;
                }
                moved = true;
            }

            if super::is_key_typed(KeyCode::Down) {
                if cursor.position.line < self.num_lines() - 1 {
                    cursor.position.line += 1;
                } else {
                    cursor.position.column = self.length_of_line(cursor.position.line).unwrap();
                }
                moved = true;
            }

            if super::is_key_typed(KeyCode::Home) {
                cursor.position.column = 0;
                moved = true;
            }

            if super::is_key_typed(KeyCode::End) {
                cursor.position.column = self.length_of_line(cursor.position.line).unwrap();
                moved = true;
            }

            if super::is_key_typed(KeyCode::PageUp) {
                cursor.position.line =
                    (cursor.position.line).saturating_sub(page_height.saturating_sub(1));
                moved = true;
                follow_slowly = true;
            }

            if super::is_key_typed(KeyCode::PageDown) {
                cursor.position.line = (cursor.position.line + page_height.saturating_sub(1))
                    .min(self.num_lines() - 1);
                moved = true;
                follow_slowly = true;
            }

            if moved {
                let shift = input::is_key_down(KeyCode::LeftShift)
                    || input::is_key_down(KeyCode::RightShift);

                if shift {
                    if cursor.end.is_none() {
                        cursor.end = Some(self.cursors[i].start);
                    }
                } else {
                    cursor.end = None;
                }
                cursor.index = self.index_of_position(cursor.position).unwrap();

                if (input::is_key_down(KeyCode::LeftAlt) || input::is_key_down(KeyCode::RightAlt))
                    && !shift
                {
                    if !self.cursors.contains(&cursor) {
                        self.cursors.push(cursor);
                    }

                    self.cursors[i].end = None;
                } else {
                    self.cursors[i] = cursor;
                }
            }

            moved_any_cursor |= moved;
        }

        (moved_any_cursor, follow_slowly)
    }

    /// Returns `(moved any cursor, follow slowly, typed, seperate edits in history)`
    #[must_use]
    pub fn type_from_input_characters(&mut self) -> (bool, bool, bool, bool) {
        /// HACK: Macroquad's clipboard interface often fails to set the clipboard, so values written to
        /// the clipboard are stored internally in case that happens. When it happens, the clipboard is
        /// emptied.
        static INTERNAL_CLIPBOARD: Mutex<String> = Mutex::new(String::new());

        let mut moved = false;
        let mut follow_slowly = false;
        let mut typed = false;
        let mut seperate_edits_in_history = false;

        let mut copied = Vec::new();
        let pasted = LazyCell::new(|| {
            let external_clipboard =
                macroquad::miniquad::window::clipboard_get().unwrap_or_default();

            if external_clipboard.is_empty() {
                INTERNAL_CLIPBOARD.lock().unwrap().clone()
            } else {
                external_clipboard
            }
        });

        let pasted_lines = OnceCell::new();
        let cursors = OnceCell::new();

        let mut characters = Vec::new();

        while let Some(character) = input::get_char_pressed() {
            characters.push(character);
        }

        for mut character in characters.into_iter().rev() {
            if character == '\r' {
                character = '\n';
            }

            'cursor: for i in 0..self.cursors.len() {
                let mut cursor = self.cursors[i];

                cursor.position = self.constrain_position_to_contents(cursor.position);

                self.cursors[i] = cursor;
                let cursor = cursor;

                match character {
                    '\u{8}' => {
                        if cursor.index > 0 || cursor.end.is_some() {
                            // Backspace
                            let range = if cursor.end.is_some() {
                                seperate_edits_in_history = true;
                                cursor.position_range()
                            } else {
                                self.move_position_left(cursor.position, 1, true)..cursor.position
                            };

                            self.remove(range).unwrap();

                            if cursor.end.is_some() {
                                self.cursors[i].end = None;
                            }

                            typed = true;
                            moved = true;
                        }
                    }
                    // NOTE: The last character is always a newline, which has a length of 1
                    '\u{7f}' => {
                        if cursor.index < self.text.len() - 1 || cursor.end.is_some() {
                            // Delete
                            let range = if cursor.end.is_some() {
                                seperate_edits_in_history = true;
                                cursor.position_range()
                            } else {
                                cursor.position..self.move_position_right(cursor.position, 1, true)
                            };

                            self.remove(range).unwrap();

                            if cursor.end.is_some() {
                                self.cursors[i].end = None;
                            }

                            typed = true;
                            moved = true;
                        }
                    }
                    _ if !character.is_control() || character == '\n' => {
                        if input::is_key_down(KeyCode::LeftControl)
                            || input::is_key_down(KeyCode::RightControl)
                        {
                            let character = character.to_ascii_uppercase();

                            // Control keybind
                            seperate_edits_in_history = true;

                            match character {
                                'A' => {
                                    // Select all
                                    let end = self.text.len() - 1;

                                    self.cursors = vec![Cursor {
                                        start: CursorLocation {
                                            position: self.position_of_index(end).unwrap(),
                                            index: end,
                                        },
                                        end: Some(CursorLocation::default()),
                                    }];

                                    moved = true;
                                    follow_slowly = true;

                                    break 'cursor;
                                }
                                'Z' => {
                                    // Undo
                                    self.undo();

                                    typed = true;

                                    break 'cursor;
                                }
                                'Y' => {
                                    // Redo
                                    self.redo();

                                    typed = true;

                                    break 'cursor;
                                }
                                'C' => {
                                    // Copy
                                    copied.push((self.text[cursor.index_range()].to_owned(), i));
                                }
                                'X' => {
                                    // Cut
                                    cursors.get_or_init(|| self.cursors.clone());

                                    copied.push((self.text[cursor.index_range()].to_owned(), i));

                                    self.remove(cursor.position_range()).unwrap();

                                    typed = true;
                                    moved = true;
                                }
                                'V' => {
                                    // Paste
                                    if !pasted.is_empty() || cursor.end.is_some() {
                                        let pasted_lines = pasted_lines.get_or_init(|| {
                                            if self.cursors.len() == 1 {
                                                return None;
                                            }

                                            let lines = pasted
                                                .lines()
                                                .take(self.cursors.len() + 1)
                                                .collect::<Vec<_>>();

                                            if lines.len() != self.cursors.len() {
                                                return None;
                                            }

                                            let mut cursor_ordering = self
                                                .cursors
                                                .iter()
                                                .enumerate()
                                                .map(|(i, cursor)| (cursor.index, i))
                                                .collect::<Vec<_>>();

                                            cursor_ordering.sort_by_key(|&(index, _)| index);

                                            let mut reordered_lines = vec![""; lines.len()];

                                            for (line, (_, i)) in
                                                cursor_ordering.into_iter().enumerate()
                                            {
                                                reordered_lines[i] = lines[line];
                                            }

                                            Some(reordered_lines)
                                        });

                                        let contents = if let Some(pasted_lines) = &pasted_lines {
                                            pasted_lines[i]
                                        } else {
                                            &pasted
                                        };

                                        self.replace(cursor.position_range(), contents).unwrap();

                                        self.cursors[i].end = None;

                                        typed = true;
                                        moved = true;
                                    }
                                }
                                _ => (),
                            }
                        } else {
                            // Typed character
                            let location = cursor.range().start;

                            let line_range =
                                self.byte_range_of_line(location.position.line).unwrap();

                            let character =
                                if self.text[line_range.clone()].split_once(';').is_none_or(
                                    |(before, _)| before.len() + line_range.start >= location.index,
                                ) {
                                    character.to_ascii_uppercase()
                                } else {
                                    character
                                };

                            self.replace(cursor.position_range(), &character.to_string())
                                .unwrap();

                            self.cursors[i].end = None;

                            typed = true;
                            moved = true;
                        }
                    }
                    _ => (),
                }
            }
        }

        if !copied.is_empty() {
            let mut copied_string = String::new();

            let cursors = cursors.get().unwrap_or(&self.cursors);
            copied.sort_by_key(|&(_, i)| cursors[i].index);

            let multi_select = copied.len() > 1;

            for (element, _) in copied {
                copied_string.push_str(&element);

                if multi_select && !element.contains('\n') {
                    copied_string.push('\n');
                }
            }

            if !copied_string.is_empty() {
                macroquad::miniquad::window::clipboard_set(&copied_string);

                *INTERNAL_CLIPBOARD.lock().unwrap() = copied_string;
            }
        }

        (moved, follow_slowly, typed, seperate_edits_in_history)
    }
}

/// Should represent a byte offset immediately after a newline
#[derive(Clone, Debug)]
pub struct Line {
    pub byte_offset: usize,
    pub colors: Vec<LineSegmentColor>,
}

impl Line {
    pub fn from_byte_offset(byte_offset: usize) -> Self {
        Self {
            byte_offset,
            colors: Vec::new(),
        }
    }

    pub fn update_colors_from(&mut self, line_contents: &str) {
        self.colors.clear();

        let offset_of_comment = line_contents
            .chars()
            .take_while(|&character| character != ';')
            .map(char::len_utf8)
            .sum();

        if offset_of_comment < line_contents.len() {
            self.colors.push(LineSegmentColor {
                relative_offset: offset_of_comment,
                color_choice: ColorChoice::Comment,
            });
        }
    }
}

/// Represents that characters on and after `relative_offset` (relative to the start
/// of the line) should be colored with `color_choice`
#[derive(Clone, Copy, Debug)]
pub struct LineSegmentColor {
    pub relative_offset: usize,
    pub color_choice: ColorChoice,
}

#[derive(Clone, Copy, Debug)]
pub enum ColorChoice {
    Default,
    Comment,
}

impl Default for ColorChoice {
    fn default() -> Self {
        Self::Default
    }
}

impl From<ColorChoice> for Color {
    fn from(value: ColorChoice) -> Self {
        match value {
            ColorChoice::Default => colors::WHITE,
            ColorChoice::Comment => colors::GRAY,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CharacterPosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq)]
pub struct Cursor {
    pub start: CursorLocation,
    pub end: Option<CursorLocation>,
}

impl Cursor {
    #[must_use]
    pub fn range(&self) -> Range<CursorLocation> {
        if let Some(end) = self.end {
            if end.index > self.index {
                self.start..end
            } else {
                end..self.start
            }
        } else {
            self.start..self.start
        }
    }

    #[must_use]
    pub fn position_range(&self) -> Range<CharacterPosition> {
        if let Some(end) = self.end {
            if end.index > self.index {
                self.start.position..end.position
            } else {
                end.position..self.start.position
            }
        } else {
            self.start.position..self.start.position
        }
    }

    #[must_use]
    pub fn index_range(&self) -> Range<usize> {
        if let Some(end) = self.end {
            if end.index > self.index {
                self.start.index..end.index
            } else {
                end.index..self.start.index
            }
        } else {
            self.start.index..self.start.index
        }
    }
}

impl PartialEq for Cursor {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start
    }
}

#[derive(Clone, Copy, Debug, Default, Eq)]
pub struct CursorLocation {
    pub position: CharacterPosition,
    pub index: usize,
}

impl PartialEq for CursorLocation {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Deref for Cursor {
    type Target = CursorLocation;

    fn deref(&self) -> &Self::Target {
        &self.start
    }
}

impl DerefMut for Cursor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.start
    }
}

#[derive(Clone, Debug, Default)]
pub struct EditHistory {
    buffer: Vec<Edit>,
    entries: Vec<EditGroup>,
    next_entry: usize,
    group_next_edit: bool,
}

impl EditHistory {
    pub fn add_edit_to_buffer(&mut self, edit: Edit) {
        if edit.replaced == edit.inserted {
            return;
        }

        self.buffer.push(edit);
    }

    pub fn insert_buffered_edits(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        if self.group_next_edit {
            self.entries[self.next_entry].edits.append(&mut self.buffer);
        } else {
            self.entries.truncate(self.next_entry);
            self.entries.push(EditGroup {
                edits: std::mem::take(&mut self.buffer),
            });

            self.group_next_edit = true;
        }
    }

    pub fn insert_edit(&mut self, edit: Edit) {
        if edit.replaced == edit.inserted {
            return;
        }

        if self.group_next_edit {
            self.entries[self.next_entry].edits.push(edit);
        } else {
            self.entries.truncate(self.next_entry);
            self.entries.push(EditGroup { edits: vec![edit] });

            self.group_next_edit = true;
        }
    }

    pub fn finish_edit_group(&mut self) {
        if self.group_next_edit {
            self.group_next_edit = false;
            self.next_entry += 1;
        }
    }

    pub fn undo(&mut self) -> Option<EditGroup> {
        self.finish_edit_group();

        if self.next_entry > 0 {
            self.next_entry -= 1;
            Some(self.entries[self.next_entry].clone())
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<EditGroup> {
        self.finish_edit_group();

        if self.next_entry < self.entries.len() {
            let entry = self.entries[self.next_entry].clone();
            self.next_entry += 1;
            Some(entry)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct EditGroup {
    pub edits: Vec<Edit>,
}

#[derive(Clone, Debug)]
pub struct Edit {
    pub start: usize,
    pub inserted: String,
    pub replaced: String,
}
