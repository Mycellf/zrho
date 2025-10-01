use std::fmt::Debug;

use ggez::{
    Context,
    graphics::{Canvas, Image},
};
use nalgebra::{Point2, Vector2};
use slotmap::{SlotMap, new_key_type};

#[derive(Debug, Default)]
pub struct WindowSet {
    pub windows: SlotMap<WindowKey, Window>,
    pub ordering: Vec<WindowKey>,
}

new_key_type! {
    pub struct WindowKey;
}

impl WindowSet {
    pub fn update(&mut self, ctx: &mut Context) {}

    pub fn draw(&mut self, ctx: &mut Context) {}
}

#[derive(Debug)]
pub struct Window {
    pub elements: Vec<Box<dyn WindowElement>>,
    pub position: Point2<f32>,
    pub size: Vector2<f32>,
    pub render_target: Image,
}

impl Window {
    pub fn update(&mut self, ctx: &mut Context) {}

    pub fn draw(&mut self, ctx: &mut Context) {}
}

pub trait WindowElement: Debug {
    fn update(&mut self, ctx: &mut Context);

    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, offset: Vector2<f32>);

    fn size(&mut self) -> Vector2<f32>;
}

pub enum UpdateAction {
    WindowDrag { grab: bool },
}
