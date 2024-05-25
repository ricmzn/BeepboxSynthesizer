use godot::prelude::*;

use crate::js_bridge::misc::performance::start_performance_timer;

pub fn initialize_v8() {
    godot_print!("v8 init: new_default_platform");
    let platform = v8::new_default_platform(0, false).make_shared();
    godot_print!("v8 init: initialize_platform");
    v8::V8::initialize_platform(platform);
    godot_print!("v8 init: initialize");
    v8::V8::initialize();
    godot_print!("v8 init: complete");
    start_performance_timer();
}
