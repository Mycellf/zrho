use std::{fmt::Debug, ops::Deref};

use ggez::{
    Context,
    graphics::{Canvas, Color, Image},
};
use nalgebra::{Point2, Vector2};
use slotmap::{SlotMap, new_key_type};

use crate::ui2::state::GlobalInput;

new_key_type! {
    pub struct WindowKey;
    pub struct ElementKey;
}

#[derive(Debug, Default)]
pub struct WindowSet {
    pub windows: SlotMap<WindowKey, Window>,
    pub ordering: Vec<WindowKey>,
}

impl WindowSet {
    pub fn update(&mut self, ctx: &mut Context, input: &GlobalInput) {
        let mut mouse_focus = true;
        let mut new_front_window = None;

        for (i, &key) in self.ordering.iter().enumerate() {
            let window = &mut self.windows[key];
            let info = WindowUpdateInfo {
                input,
                mouse_focus: mouse_focus && window.contains_point(input.mouse_position),
            };

            if info.mouse_focus {
                mouse_focus = false;
            }

            for event in window.update(ctx, info) {
                match event {
                    WindowUpdateEvent::MoveToFront => {
                        if new_front_window.is_none() {
                            new_front_window = Some(i);
                        }
                    }
                }
            }
        }

        if let Some(i) = new_front_window {
            let key = self.ordering.remove(i);
            self.ordering.insert(0, key);
        }
    }

    pub fn draw(&mut self, ctx: &mut Context) {
        let mut canvas = Canvas::from_frame(ctx, Some(Color::BLACK));

        for &key in self.ordering.iter().rev() {
            let window = &mut self.windows[key];
            let info = WindowDrawInfo {
                canvas: &mut canvas,
            };

            window.draw(ctx, info);
        }

        canvas.finish(ctx).unwrap();
    }
}

#[derive(Debug)]
pub struct Window {
    pub elements: SlotMap<ElementKey, WindowElementEntry>,
    pub ordering: Vec<ElementKey>,
    pub position: Point2<f32>,
    pub size: Vector2<f32>,
    pub render_target: Image,
}

impl Window {
    pub fn update(&mut self, ctx: &mut Context, info: WindowUpdateInfo) -> Vec<WindowUpdateEvent> {
        Vec::new()
    }

    pub fn draw(&mut self, ctx: &mut Context, info: WindowDrawInfo) {}

    pub fn contains_point(&self, point: Point2<f32>) -> bool {
        let offset = point - self.position;
        offset.x >= 0.0 && offset.y >= 0.0 && offset.x < self.size.x && offset.y < self.size.y
    }
}

#[derive(Debug)]
pub struct WindowElementEntry {
    pub element: Box<dyn WindowElement>,
}

pub trait WindowElement: Debug {
    fn update(&mut self, ctx: &mut Context, info: ElementUpdateInfo) -> Vec<ElementUpdateEvent>;

    fn draw(&mut self, ctx: &mut Context, info: ElementDrawInfo);
}

pub enum WindowUpdateEvent {
    MoveToFront,
}

pub struct WindowUpdateInfo<'a> {
    pub input: &'a GlobalInput,
    pub mouse_focus: bool,
}

pub struct WindowDrawInfo<'a> {
    pub canvas: &'a mut Canvas,
}

pub enum ElementUpdateEvent {
    WindowDrag { grab: bool },
    TakeKeyboardFocus,
    SetHeight { new_height: f32 },
}

pub struct ElementUpdateInfo<'a> {
    pub input: ElementInput<'a>,
    pub keyboard_focus: bool,
    pub mouse_focus: bool,
    pub size: Vector2<f32>,
}

pub struct ElementDrawInfo<'a> {
    pub canvas: &'a mut Canvas,
    pub offset: Vector2<f32>,
    pub size: Vector2<f32>,
}

pub struct ElementInput<'a> {
    pub mouse_position: Vector2<f32>,
    pub global_input: &'a GlobalInput,
}

impl<'a> Deref for ElementInput<'a> {
    type Target = GlobalInput;

    fn deref(&self) -> &Self::Target {
        self.global_input
    }
}
