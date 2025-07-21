use macroquad::{
    camera::{self, Camera2D},
    color::{Color, colors},
    input::{self, MouseButton},
    math::Vec2,
    text::{self, TextParams},
    texture,
};

use super::{
    FONT, FONT_ASPECT, FONT_VERTICAL_OFFSET,
    element::{DrawArea, Element, ElementEntry, UpdateContext, UpdateResult},
    theme::Theme,
};

#[non_exhaustive]
#[derive(Debug)]
pub struct Window {
    pub position: Vec2,
    pub grab_position: Option<Vec2>,
    pub element_grabbing_mouse: bool,
    pub is_focused: bool,

    pub size: Vec2,
    pub camera: Camera2D,
    pub elements: Vec<ElementEntry>,
    pub update_all: bool,
    pub update_focus: bool,
    pub update_any: bool,

    pub theme: Theme,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct WindowContext {
    pub position: Vec2,
    pub is_grabbed: bool,
    pub theme: Theme,
    pub is_focused: bool,
}

impl WindowContext {
    #[must_use]
    pub fn mouse_position(&self) -> Vec2 {
        super::mouse_position() - self.position
    }
}

impl Window {
    pub const RESOLUTION_UPSCALING: f32 = 4.0;
    pub const BASE_TEXT_HEIGHT: f32 = 15.0;
    pub const BASE_TEXT_WIDTH: f32 = Self::BASE_TEXT_HEIGHT * FONT_ASPECT;

    #[must_use]
    pub fn new(position: Vec2, width: f32, is_focused: bool) -> Self {
        Self {
            position,
            grab_position: None,
            element_grabbing_mouse: false,
            is_focused,

            size: Vec2::new(width, 0.0),
            camera: Camera2D {
                offset: Vec2::new(-1.0, -1.0),
                ..Default::default()
            },
            elements: Vec::new(),
            update_all: false,
            update_focus: false,
            update_any: false,

            theme: Theme::default(),
        }
    }

    pub fn push_element(&mut self, element: impl Element) {
        let height = element.height();

        self.elements.push(ElementEntry::new(element));

        if height > 0.0 {
            self.size.y += height;

            self.resize_texture();
            self.deduplicate_keyboard_focus();
        }
    }

    pub fn update_height(&mut self) {
        let new_height: f32 = self.elements.iter().map(|entry| entry.height()).sum();

        if new_height != self.size.y {
            self.size.y = new_height;

            self.resize_texture();
        }
    }

    pub fn resize_texture(&mut self) {
        let target_size = self.size * Self::RESOLUTION_UPSCALING;

        self.camera.render_target = Some(texture::render_target(
            target_size.x as u32,
            target_size.y as u32,
        ));

        self.camera.zoom = 2.0 / self.size;

        self.update_all = true;
    }

    #[must_use]
    pub fn context(&self) -> WindowContext {
        WindowContext {
            position: self.position,
            is_grabbed: self.grab_position.is_some(),
            theme: self.theme,
            is_focused: self.is_focused,
        }
    }

    #[must_use]
    pub fn screen_area(&self) -> DrawArea {
        DrawArea {
            offset: self.position * super::scaling_factor(),
            size: self.size * super::scaling_factor(),
        }
    }

    #[must_use]
    pub fn local_area(&self) -> DrawArea {
        DrawArea {
            offset: Vec2::ZERO,
            size: self.size,
        }
    }

    /// Returns whether or not the window was clicked
    pub fn update(&mut self, previous_hovered_window: &mut bool) -> bool {
        if let Some(grab_position) = self.grab_position {
            self.position = super::mouse_position() - grab_position;
        }

        let context = self.context();

        let mut cumulative_height = 0.0;

        let mut should_grab_window = false;
        let mut should_grab_mouse = false;

        let mut should_take_keyboard_focus = None;

        for (i, element) in self.elements.iter_mut().enumerate() {
            let height = element.height();

            let area = DrawArea {
                offset: Vec2::new(0.0, cumulative_height),
                size: Vec2::new(self.size.x, height),
            };

            element.focus.mouse_hover = !self.element_grabbing_mouse
                && !*previous_hovered_window
                && area.contains_point(context.mouse_position());

            let update_context = UpdateContext::new(context, area, element);

            let UpdateResult {
                update_graphics,
                grab_mouse,
                grab_window,
                take_keyboard_focus,
            } = element.update(update_context);

            element.needs_update |= update_graphics;
            self.update_any |= update_graphics;

            element.focus.mouse_grab = grab_mouse;
            should_grab_mouse |= grab_mouse;

            should_grab_window |= grab_window;

            let clicked =
                element.focus.mouse_hover && input::is_mouse_button_pressed(MouseButton::Left);

            if (clicked || take_keyboard_focus) && element.uses_keyboard_focus() {
                should_take_keyboard_focus = Some(i);
            }

            cumulative_height += height;
        }

        self.grab_position = should_grab_window.then(|| context.mouse_position());

        self.element_grabbing_mouse = should_grab_mouse;

        if let Some(index) = should_take_keyboard_focus {
            if !self.elements[index].focus.keyboard {
                self.clear_keyboard_focus();
                let element = &mut self.elements[index];
                element.focus.keyboard = true;
                element.needs_update = true;
                self.update_any = true;
            }
        }

        if self.element_grabbing_mouse
            || self.local_area().contains_point(context.mouse_position())
                && !*previous_hovered_window
        {
            *previous_hovered_window = true;

            input::is_mouse_button_pressed(MouseButton::Left)
        } else {
            false
        }
    }

    /// Ignores the topmost item that has keyboard focus, clears the rest.
    pub fn deduplicate_keyboard_focus(&mut self) -> bool {
        let mut found_focused_element = false;

        for element in &mut self.elements {
            if found_focused_element {
                element.needs_update = true;
                self.update_any = true;

                element.focus.keyboard = false;
            }

            if element.focus.keyboard {
                found_focused_element = true;
            }
        }

        found_focused_element
    }

    pub fn clear_keyboard_focus(&mut self) -> bool {
        let mut found_focused_element = false;

        for element in &mut self.elements {
            if element.focus.keyboard {
                element.needs_update = true;
                self.update_any = true;

                element.focus.keyboard = false;
                found_focused_element = true;
            }
        }

        found_focused_element
    }

    pub fn draw(&mut self) {
        self.update_texture();

        self.screen_area()
            .draw_rectangle(self.theme.background_color);

        self.screen_area().draw_texture(
            &self.camera.render_target.as_ref().unwrap().texture,
            colors::WHITE,
        );
    }

    pub fn update_texture(&mut self) {
        if !(self.update_any || self.update_all || self.update_focus) || self.size.y <= 0.0 {
            return;
        }

        camera::push_camera_state();
        camera::set_camera(&self.camera);

        let context = self.context();

        let mut cumulative_height = 0.0;

        for element in &mut self.elements {
            let height = element.height();

            if element.needs_update
                || self.update_all
                || self.update_focus && element.uses_window_focus().is_usesd(element.focus.keyboard)
            {
                let area = DrawArea {
                    offset: Vec2::new(0.0, cumulative_height),
                    size: Vec2::new(self.size.x, height),
                };

                let update_context = UpdateContext::new(context, area, element);

                element.draw(update_context);
                element.needs_update = false;
            }

            cumulative_height += height;
        }

        self.update_all = false;
        self.update_focus = false;
        self.update_any = false;

        camera::pop_camera_state();
    }
}

#[must_use]
pub fn text_params_with_height(height: f32) -> TextParams<'static> {
    TextParams {
        font: Some(&FONT),
        font_size: (height * Window::RESOLUTION_UPSCALING) as u16,
        font_scale: 1.0 / Window::RESOLUTION_UPSCALING,
        font_scale_aspect: 1.0,
        rotation: 0.0,
        color: colors::WHITE,
    }
}

pub fn draw_text_with_size(text: &str, position: Vec2, height: f32, color: Color) {
    text::draw_text_ex(
        text,
        position.x,
        position.y + height * FONT_VERTICAL_OFFSET,
        TextParams {
            color,
            ..text_params_with_height(height)
        },
    );
}
