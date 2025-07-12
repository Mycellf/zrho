use std::ops::Range;

use macroquad::{
    color::{Color, colors},
    math::Vec2,
    text::{self, TextDimensions, TextParams},
};

use crate::interface::window::EditorWindow;

#[derive(Clone, Debug)]
pub struct TextEditor {
    pub text: String,
    pub lines: Vec<Line>,
    pub cursors: Vec<Cursor>,
}

impl TextEditor {
    pub fn new(text: String) -> Self {
        let lines = Self::line_indecies_from(&text);
        let cursors = vec![Cursor::new()];

        let mut result = Self {
            text,
            lines,
            cursors,
        };

        result.update_colors_of_all_lines();

        result
    }

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

    pub fn draw_all(&self, position: Vec2, text_size: f32, line_height: f32, character_width: f32) {
        self.draw_range(
            0..self.lines.len(),
            position,
            text_size,
            line_height,
            character_width,
        );
    }

    pub fn draw_range(
        &self,
        lines: Range<usize>,
        mut position: Vec2,
        text_size: f32,
        line_height: f32,
        character_width: f32,
    ) {
        let line_height = line_height * text_size;

        for i in lines {
            let segments = self.color_segments_of_line(i).unwrap();

            position.y += line_height;
            let mut line_position = position - Vec2::Y * line_height * 0.125;

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
                        ..EditorWindow::text_params_with_size(text_size)
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

        let removed_bytes = end_index.checked_sub(start_index)?;
        let removed_lines = range.end.line.checked_sub(range.start.line)?;

        for moved_line in &mut self.lines[range.end.line + 1..] {
            moved_line.byte_offset += text.len() - removed_bytes;
        }

        let mut new_lines = Vec::new();

        for (i, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                new_lines.push(Line::from_byte_offset(start_index + i + 1));
            }
        }

        let num_new_lines = new_lines.len();

        self.lines
            .splice(range.start.line..range.end.line, new_lines);

        self.text.replace_range(start_index..end_index, text);

        for i in 0..self.cursors.len() {
            let cursor = &mut self.cursors[i];

            if cursor.index > start_index {
                if cursor.index < end_index {
                    cursor.index = start_index;
                } else {
                    cursor.index -= removed_bytes;
                }

                cursor.index += text.len();

                if cursor.position.line <= range.end.line {
                    let index = cursor.index;
                    self.cursors[i].position = self.position_of_index(index).unwrap();
                } else {
                    cursor.position.line += num_new_lines;
                    cursor.position.line -= removed_lines;
                }
            }
        }

        Some(())
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
            .scan(0, |acc, character| {
                *acc += character.len_utf8();

                assert!(*acc <= index, "Index {index} is inside a codepoint");

                (*acc < index).then_some(())
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

#[derive(Clone, Copy, Debug)]
pub struct CharacterPosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Cursor {
    pub position: CharacterPosition,
    pub index: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            position: CharacterPosition { line: 0, column: 0 },
            index: 0,
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}
