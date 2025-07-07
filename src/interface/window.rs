use macroquad::{color::Color, math::Vec2, shapes};

use crate::simulation::{
    computer::Computer,
    program::{Program, ProgramAssemblyError},
};

use super::text_editor::TextEditor;

pub struct EditorWindow {
    pub position: Vec2,
    pub size: Vec2,
    pub name: String,

    pub text_editor: TextEditor,
    pub program: Result<Program, Vec<ProgramAssemblyError>>,
}

impl EditorWindow {
    pub const BACKGROUND: Color = Color::from_hex(0x202030);

    pub fn assemble_program(&mut self, target_computer: &Computer) {
        self.program =
            Program::assemble_from(self.name.clone(), &self.text_editor.text, target_computer);
    }

    pub fn draw(&self) {
        shapes::draw_rectangle(
            self.position.x,
            self.position.y,
            self.size.x,
            self.size.y,
            Self::BACKGROUND,
        );
    }
}
