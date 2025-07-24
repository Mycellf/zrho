use macroquad::{
    color::{Color, colors},
    input::{self, MouseButton},
    math::Vec2,
    shapes,
};

use crate::ui::{
    self,
    element::{Element, UpdateContext, UpdateResult, WindowFocusUse},
};

pub trait ScrollableElement: Element {
    fn set_scroll(&mut self, scroll: f32);

    #[must_use]
    fn get_scroll(&self) -> f32;

    #[must_use]
    fn maximum_scroll(&self) -> f32;

    #[must_use]
    fn page_height(&self) -> f32 {
        self.height()
    }

    #[must_use]
    fn scrollable_height(&self) -> f32 {
        self.maximum_scroll() + self.page_height()
    }

    #[must_use]
    fn page_offset(&self) -> f32 {
        0.0
    }
}

#[derive(Debug)]
pub struct ScrollBar {
    pub inner: Box<dyn ScrollableElement>,
    pub state: Option<ScrollBarState>,
}

impl Element for ScrollBar {
    fn height(&self) -> f32 {
        self.inner.height()
    }

    fn uses_keyboard_focus(&self) -> bool {
        self.inner.uses_keyboard_focus()
            && self.state.is_none_or(|state| state.grab_position.is_none())
    }

    fn uses_window_focus(&self) -> WindowFocusUse {
        self.inner.uses_window_focus()
    }

    fn update(&mut self, context: UpdateContext) -> UpdateResult {
        let mut result = UpdateResult::default();

        if self.inner.maximum_scroll() > 0.0 {
            if self.state.is_none() {
                self.state = Some(ScrollBarState::default());
                result.update_graphics = true;
            }

            let state = self.state.as_mut().unwrap();

            result |= state.update_selection(&*self.inner, context);
        } else if self.state.is_some() {
            self.state = None;
            result.update_graphics = true;
        }

        let mut inner_context = context;

        if result.grab_mouse {
            inner_context.focus.mouse_hover = false;
            inner_context.focus.mouse_grab = false;
        }

        result |= self.inner.update(inner_context);

        if let Some(state) = &mut self.state {
            result |= state.update_scrolling(&mut *self.inner, context);
        }

        result
    }

    fn draw(&mut self, context: UpdateContext) {
        let UpdateContext { area, .. } = context;

        self.inner.draw(context);

        if let Some(scroll_bar) = &self.state {
            shapes::draw_rectangle(
                area.offset.x + area.size.x - scroll_bar.size.x,
                area.offset.y + scroll_bar.vertical_offset + self.inner.page_offset(),
                scroll_bar.size.x,
                scroll_bar.size.y,
                scroll_bar.color(),
            );
        }
    }

    fn force_update(&mut self) {
        self.inner.force_update();
    }
}

impl ScrollBar {
    pub fn new(inner: impl ScrollableElement) -> Self {
        Self {
            inner: Box::new(inner),
            state: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollBarState {
    pub size: Vec2,
    pub vertical_offset: f32,
    pub is_selected: bool,
    pub grab_position: Option<f32>,
    pub color: f32,
}

impl ScrollBarState {
    pub const BASE_COLOR: Color = colors::WHITE;
    pub const ALTERNATE_COLOR: Color = colors::LIGHTGRAY;

    pub const SELECTED_WIDTH: f32 = 7.5;
    pub const UNSELECTED_WIDTH: f32 = 2.5;
    pub const MAX_WIDTH: f32 = Self::SELECTED_WIDTH.max(Self::UNSELECTED_WIDTH);
    pub const MIN_WIDTH: f32 = Self::SELECTED_WIDTH.min(Self::UNSELECTED_WIDTH);

    pub const PADDING: f32 = 2.5;
    pub const MAX_PADDED_WIDTH: f32 = Self::MAX_WIDTH + Self::PADDING;

    pub const MINIMUM_HEIGHT: f32 = 40.0;

    #[must_use]
    pub fn color(&self) -> Color {
        ui::color_lerp(Self::BASE_COLOR, Self::ALTERNATE_COLOR, self.color)
    }

    #[must_use]
    pub fn update_selection(
        &mut self,
        element: &dyn ScrollableElement,
        context: UpdateContext,
    ) -> UpdateResult {
        let UpdateContext {
            window,
            area,
            focus,
        } = context;

        let mut result = UpdateResult::default();

        let mouse_position =
            window.mouse_position() - area.offset - Vec2::new(0.0, element.page_offset());

        let area_hovered = self.grab_position.is_some()
            || focus.mouse_hover && mouse_position.x > area.size.x - Self::MAX_PADDED_WIDTH;

        result.grab_mouse |= area_hovered
            && (input::is_mouse_button_down(MouseButton::Left)
                || input::is_mouse_button_pressed(MouseButton::Left));

        let target_width = if area_hovered {
            Self::SELECTED_WIDTH
        } else {
            Self::UNSELECTED_WIDTH
        };

        let frame_time = macroquad::time::get_frame_time();

        let next_width = ui::exp_decay_cutoff(self.size.x, target_width, 25.0, frame_time, 0.05).0;

        self.size.x = next_width;

        result.update_graphics |= self.size.x != target_width;

        result
    }

    pub fn update_scrolling(
        &mut self,
        element: &mut dyn ScrollableElement,
        context: UpdateContext,
    ) -> UpdateResult {
        let UpdateContext {
            window,
            area,
            focus,
        } = context;

        let mut result = UpdateResult::default();

        let height =
            (element.page_height().powi(2) / element.scrollable_height()).max(Self::MINIMUM_HEIGHT);

        if self.size.y != height {
            self.size.y = height;

            result.update_graphics = true;
        }

        let mouse_position =
            window.mouse_position() - area.offset - Vec2::new(0.0, element.page_offset());

        let area_hovered = self.grab_position.is_some()
            || focus.mouse_hover && mouse_position.x > area.size.x - Self::MAX_PADDED_WIDTH;

        let bar_hovered = self.grab_position.is_some()
            || area_hovered
                && mouse_position.y > self.vertical_offset
                && mouse_position.y < self.vertical_offset + self.size.y;

        let frame_time = macroquad::time::get_frame_time();

        let target_color = (bar_hovered && self.grab_position.is_none()) as usize as f32;

        let next_color = ui::exp_decay_cutoff(self.color, target_color, 50.0, frame_time, 0.05).0;

        self.color = next_color;

        result.update_graphics |= self.color != target_color;

        let scroll_proportion = element.get_scroll() / element.maximum_scroll();

        let new_vertical_offset = scroll_proportion * (element.page_height() - self.size.y);

        if self.vertical_offset != new_vertical_offset {
            self.vertical_offset = new_vertical_offset;
            result.update_graphics = true;
        }

        if area_hovered {
            if let Some(grab_position) = self.grab_position {
                if input::is_mouse_button_down(MouseButton::Left) {
                    let new_vertical_offset = (mouse_position.y - grab_position)
                        .clamp(0.0, element.page_height() - self.size.y);

                    result.update_graphics |=
                        self.set_vertical_offset(element, new_vertical_offset);
                } else {
                    self.grab_position = None;
                    result.update_graphics = true;
                }
            } else if input::is_mouse_button_pressed(MouseButton::Left) {
                if !bar_hovered {
                    let new_vertical_offset = (mouse_position.y - self.size.y / 2.0)
                        .clamp(0.0, element.page_height() - self.size.y);

                    result.update_graphics |=
                        self.set_vertical_offset(element, new_vertical_offset);
                }

                self.grab_position = Some(mouse_position.y - self.vertical_offset);
                result.update_graphics = true;
            }
        }

        result
    }

    #[must_use]
    pub fn set_vertical_offset(
        &mut self,
        element: &mut dyn ScrollableElement,
        new_vertical_offset: f32,
    ) -> bool {
        if self.vertical_offset != new_vertical_offset {
            self.vertical_offset = new_vertical_offset;

            let scroll_proportion = self.vertical_offset / (element.page_height() - self.size.y);

            element.set_scroll(scroll_proportion * element.maximum_scroll());

            true
        } else {
            false
        }
    }
}

impl Default for ScrollBarState {
    fn default() -> Self {
        Self {
            size: Vec2::new(Self::UNSELECTED_WIDTH, 0.0),
            vertical_offset: 0.0,
            is_selected: true,
            grab_position: None,
            color: 0.0,
        }
    }
}
