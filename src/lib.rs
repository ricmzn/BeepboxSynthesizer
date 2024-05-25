use godot::prelude::*;

use crate::js_bridge::init::initialize_v8;

mod js_bridge;
mod classes;

struct BeepBoxSynthesizer;

#[gdextension]
unsafe impl ExtensionLibrary for BeepBoxSynthesizer {
    fn on_level_init(level: InitLevel) {
        if let InitLevel::Scene = level {
            std::env::set_var("RUST_LIB_BACKTRACE", "1");
            std::env::set_var("RUST_BACKTRACE", "1");
            std::panic::set_hook(Box::new(|info| {
                godot_error!("[Panic] {info}");
            }));
            initialize_v8();
        }
    }
}
