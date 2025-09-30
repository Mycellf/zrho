use ggez::{
    Context, ContextBuilder, GameError, GameResult,
    conf::{Backend, Conf, FullscreenType, WindowMode, WindowSetup},
    event::{self, EventHandler},
    graphics::{Canvas, Color, Sampler},
    input::keyboard::KeyInput,
    winit::keyboard::{Key, NamedKey},
};

use crate::ui2::{DRAW_FPS, START_IN_FULLSCREEN, utils};

pub struct State {
    fullscreen: bool,
    was_maximized: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            fullscreen: START_IN_FULLSCREEN,
            was_maximized: START_IN_FULLSCREEN,
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
        repeated: bool,
    ) -> Result<(), GameError> {
        if !repeated {
            if input.event.logical_key == Key::Named(NamedKey::Escape) {
                ctx.request_quit();
            }

            if input.event.logical_key == Key::Named(NamedKey::F11) {
                if self.fullscreen {
                    ctx.gfx.set_fullscreen(FullscreenType::Windowed).unwrap();
                    ctx.gfx.window().set_maximized(self.was_maximized);
                } else {
                    self.was_maximized = ctx.gfx.window().is_maximized();
                    ctx.gfx.set_fullscreen(FullscreenType::Desktop).unwrap();
                }

                self.fullscreen ^= true;
            }
        }

        Ok(())
    }
}
