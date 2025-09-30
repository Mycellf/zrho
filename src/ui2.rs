pub mod state;
pub mod utils;

use std::{env, sync::LazyLock};

const START_IN_FULLSCREEN: bool = true;

static DRAW_FPS: LazyLock<bool> =
    LazyLock::new(|| env::args().skip(1).any(|argument| argument == "--draw-fps"));
