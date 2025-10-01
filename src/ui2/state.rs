use ggez::{
    Context, ContextBuilder, GameResult,
    conf::{Backend, Conf, FullscreenType, WindowMode, WindowSetup},
    event::{self, EventHandler},
    graphics::{Canvas, Color, Sampler},
    input::keyboard::KeyInput,
    winit::{
        event::{KeyEvent, MouseButton},
        keyboard::{Key, KeyLocation, NamedKey},
        platform::modifier_supplement::KeyEventExtModifierSupplement,
    },
};
use nalgebra::{Point2, Vector2, point, vector};

use crate::ui2::{DRAW_FPS, START_IN_FULLSCREEN, utils};

pub struct State {
    fullscreen: bool,
    was_maximized: bool,

    input: GlobalInput,
}

impl Default for State {
    fn default() -> Self {
        Self {
            fullscreen: START_IN_FULLSCREEN,
            was_maximized: START_IN_FULLSCREEN,

            input: GlobalInput::default(),
        }
    }
}

#[derive(Debug)]
pub struct GlobalInput {
    pub keys_down: Vec<(Key, KeyLocation)>,
    pub keys_pressed: Vec<(Key, KeyLocation)>,

    pub text_input: String,

    pub mouse_in_window: bool,
    pub mouse_position: Point2<f32>,
    pub mouse_scroll: Vector2<f32>,

    pub mouse_buttons_down: Vec<MouseButton>,
    pub mouse_buttons_pressed: Vec<MouseButton>,
}

impl Default for GlobalInput {
    fn default() -> Self {
        Self {
            keys_down: Vec::new(),
            keys_pressed: Vec::new(),

            text_input: String::new(),

            mouse_in_window: true,
            mouse_position: point![0.0, 0.0],
            mouse_scroll: vector![0.0, 0.0],

            mouse_buttons_down: Vec::new(),
            mouse_buttons_pressed: Vec::new(),
        }
    }
}

impl State {
    pub fn run(self) -> GameResult {
        let state = State::default();

        let (ctx, event_loop) = ContextBuilder::new("ggez_test", "Mycellf")
            .default_conf(Conf {
                window_mode: WindowMode {
                    fullscreen_type: if state.fullscreen {
                        FullscreenType::Desktop
                    } else {
                        FullscreenType::Windowed
                    },
                    min_width: 400.0,
                    min_height: 400.0,
                    ..Default::default()
                },
                window_setup: WindowSetup {
                    title: "Ggez Test".to_owned(),
                    ..Default::default()
                },
                backend: Backend::default(),
            })
            .add_resource_path(env!("CARGO_MANIFEST_DIR").to_owned() + "/resources")
            .build()
            .unwrap();

        event::run(ctx, event_loop, state)
    }
}

impl EventHandler for State {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        println!("{:?}", self.input);

        self.input.text_input.clear();

        self.input.mouse_scroll = vector![0.0, 0.0];

        self.input.keys_pressed.clear();
        self.input.mouse_buttons_pressed.clear();
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Some(Color::BLACK));
        canvas.set_sampler(Sampler::nearest_clamp());

        if *DRAW_FPS {
            utils::draw_fps(ctx, &mut canvas);
        }

        canvas.finish(ctx).unwrap();

        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: KeyInput,
        _repeated: bool,
    ) -> GameResult {
        match input.event {
            KeyEvent {
                logical_key: Key::Named(NamedKey::Escape),
                repeat: false,
                ..
            } => {
                ctx.request_quit();
            }
            KeyEvent {
                logical_key: Key::Named(NamedKey::F11),
                repeat: false,
                ..
            } => {
                if self.fullscreen {
                    ctx.gfx.set_fullscreen(FullscreenType::Windowed).unwrap();
                    ctx.gfx.window().set_maximized(self.was_maximized);
                } else {
                    self.was_maximized = ctx.gfx.window().is_maximized();
                    ctx.gfx.set_fullscreen(FullscreenType::Desktop).unwrap();
                }

                self.fullscreen ^= true;
            }
            _ => (),
        }

        let key = (input.event.key_without_modifiers(), input.event.location);

        if !self.input.keys_down.contains(&key) {
            self.input.keys_down.push(key.clone());
        }

        if !self.input.keys_pressed.contains(&key) {
            self.input.keys_pressed.push(key);
        }

        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        let released_key = (input.event.key_without_modifiers(), input.event.location);

        self.input.keys_down.retain(|key| *key != released_key);

        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) -> GameResult {
        if !self.input.mouse_buttons_down.contains(&button) {
            self.input.mouse_buttons_down.push(button);
        }

        if !self.input.mouse_buttons_pressed.contains(&button) {
            self.input.mouse_buttons_pressed.push(button);
        }

        Ok(())
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        released_button: MouseButton,
        _x: f32,
        _y: f32,
    ) -> GameResult {
        self.input
            .mouse_buttons_down
            .retain(|button| *button != released_button);

        Ok(())
    }

    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult {
        self.input.mouse_position = point![x, y];

        Ok(())
    }

    fn mouse_enter_or_leave(&mut self, _ctx: &mut Context, entered: bool) -> GameResult {
        self.input.mouse_in_window = entered;

        Ok(())
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, x: f32, y: f32) -> GameResult {
        self.input.mouse_scroll += vector![x, y];

        Ok(())
    }

    fn text_input_event(&mut self, _ctx: &mut Context, character: char) -> GameResult {
        self.input.text_input.push(character);

        Ok(())
    }
}
