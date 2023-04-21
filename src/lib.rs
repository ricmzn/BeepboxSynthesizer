use godot::engine::class_macros::auto_register_classes;
use godot::prelude::*;
use once_cell::sync::OnceCell;
use std::time::Instant;

mod js;
mod synthesizer;
mod utils;

pub static REFERENCE_TIME: OnceCell<Instant> = OnceCell::new();

struct BeepboxSynthesizer;

struct Init;

impl ExtensionLayer for Init {
    fn initialize(&mut self) {
        auto_register_classes();
    }

    fn deinitialize(&mut self) {}
}

#[gdextension]
unsafe impl ExtensionLibrary for BeepboxSynthesizer {
    fn load_library(handle: &mut InitHandle) -> bool {
        std::env::set_var("RUST_LIB_BACKTRACE", "1");
        std::panic::set_hook(Box::new(|info| {
            godot_print!("{info}");
        }));
        REFERENCE_TIME.set(Instant::now()).unwrap();
        handle.register_layer(InitLevel::Scene, Init);
        js::init_v8();
        true
    }
}
