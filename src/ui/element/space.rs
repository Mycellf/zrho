use crate::ui::element::{Element, UpdateContext, UpdateResult};

#[derive(Debug)]
pub struct Space {
    pub height: f32,
}

impl Element for Space {
    fn height(&self) -> f32 {
        self.height
    }

    fn update(&mut self, _: UpdateContext) -> UpdateResult {
        UpdateResult::default()
    }

    fn draw(&mut self, UpdateContext { window, area, .. }: UpdateContext) {
        area.draw_rectangle(window.theme.background_color);
    }
}
