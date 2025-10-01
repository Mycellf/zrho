use std::{fmt::Debug, ops::Deref};

use ggez::{
    Context,
    graphics::{Canvas, Image},
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
    pub fn update(&mut self, ctx: &mut Context, input: &GlobalInput) {}

    pub fn draw(&mut self, ctx: &mut Context) {}
}

#[derive(Debug)]
pub struct Window {
    pub elements: SlotMap<ElementKey, Box<dyn WindowElement>>,
    pub ordering: Vec<ElementKey>,
    pub position: Point2<f32>,
    pub size: Vector2<f32>,
    pub render_target: Image,
}

impl Window {
    pub fn update(&mut self, ctx: &mut Context, input: &GlobalInput) {}

    pub fn draw(&mut self, ctx: &mut Context) {}
}

pub trait WindowElement: Debug {
    fn update(&mut self, ctx: &mut Context, info: UpdateInfo);

    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, offset: Vector2<f32>);

    fn height(&mut self) -> f32;
}

pub enum UpdateEvent {
    WindowDrag { grab: bool },
    TakeKeyboardFocus,
}

pub struct UpdateInfo {
    pub input: GlobalInput,
    pub keyboard_focus: bool,
    pub mouse_focus: bool,
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
