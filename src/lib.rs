use godot::prelude::*;
use once_cell::sync::OnceCell;
use std::time::Instant;

mod js;
mod synthesizer;
mod utils;

pub static REFERENCE_TIME: OnceCell<Instant> = OnceCell::new();

struct BeepboxSynthesizer;

#[gdextension]
unsafe impl ExtensionLibrary for BeepboxSynthesizer {
    fn on_level_init(level: InitLevel) {
        if let InitLevel::Scene = level {
            std::env::set_var("RUST_LIB_BACKTRACE", "1");
            std::panic::set_hook(Box::new(|info| {
                godot_print!("{info}");
            }));
            REFERENCE_TIME.set(Instant::now()).unwrap();
            js::init_v8();
        }
    }
}
