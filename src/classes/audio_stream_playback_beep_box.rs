use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::TryLockError::{Poisoned, WouldBlock};

use anyhow::{Context, Result};
use godot::engine::{AudioStreamPlayback, IAudioStreamPlayback};
use godot::engine::native::AudioFrame;
use godot::prelude::*;

use crate::js_bridge::{
    audio::poll_js_audio,
    util::V8ObjectHelpers,
};
use crate::js_bridge::context::JSContext;

const SYNTHESIZER_FILENAME: &str = "beepbox_synth.js";
const SYNTHESIZER_SCRIPT: &str = include_str!("../../dependencies/jummbox/website/beepbox_synth.js");

#[derive(GodotClass)]
#[class(no_init, base = AudioStreamPlayback)]
pub struct AudioStreamPlaybackBeepBox {
    base: Base<AudioStreamPlayback>,
    js_context: Option<RefCell<JSContext>>,
    buffer_left: Vec<f32>,
    buffer_right: Vec<f32>,
    mutex: Mutex<()>,
}

#[godot_api]
impl AudioStreamPlaybackBeepBox {
    #[func]
    pub fn eval(&self, src: GString) -> Variant {
        self.run_js(concat!(module_path!(), "::AudioStreamPlaybackBeepBox::eval"), &src.to_string())
    }

    pub fn new(base: Base<AudioStreamPlayback>) -> Self {
        godot_print!("AudioStreamPlaybackBeepbox.new()");

        let js_context = Self::create_js_context()
            .inspect_err(|err| godot_error!("failed to initialize javascript context: {err}"))
            .map(RefCell::new)
            .ok();

        AudioStreamPlaybackBeepBox {
            base,
            js_context,
            buffer_left: Vec::new(),
            buffer_right: Vec::new(),
            mutex: Mutex::new(()),
        }
    }

    fn create_js_context() -> Result<JSContext> {
        let mut js_context = JSContext::new()?;

        js_context.run(SYNTHESIZER_FILENAME, SYNTHESIZER_SCRIPT)
            .context("failed to run beepbox_synth.js")?;

        js_context.run("synth_init", "synth = new beepbox.Synth()")
            .context("failed to instantiate beepbox synthesizer")?;

        Ok(js_context)
    }

    fn run_js(&self, filename: &str, src: &str) -> Variant {
        if let Some(js) = self.js_context.as_ref() {
            js.borrow_mut().run(filename, src).inspect_err(|err| godot_error!("error caught in javascript: {err}")).unwrap_or_default()
        } else {
            Variant::nil()
        }
    }

    pub fn do_scoped_with_mutex(&self, filename: &str, callback: &mut dyn FnMut(&mut v8::HandleScope<'_>) -> Result<Variant>) -> Result<Variant> {
        if let Some(js_context) = self.js_context.as_ref() {
            let _guard = self.mutex.lock();
            js_context.borrow_mut().do_scoped(filename, callback)
        } else {
            Ok(Variant::nil())
        }
    }
}

#[godot_api]
impl IAudioStreamPlayback for AudioStreamPlaybackBeepBox {
    fn start(&mut self, from_pos: f64) {
        godot_print!("start({from_pos})");
        self.run_js("resume", "synth.resetEffects(); synth.play()");
    }

    fn stop(&mut self) {
        godot_print!("stop()");
        self.run_js("resume", "synth.pause()");
    }

    fn is_playing(&self) -> bool {
        godot_print!("is_playing");
        false
    }

    fn get_loop_count(&self) -> i32 {
        godot_print!("get_loop_count()");
        0
    }

    fn get_playback_position(&self) -> f64 {
        godot_print!("get_playback_position()");
        0.0
    }

    fn seek(&mut self, position: f64) {
        godot_print!("seek({position})");
    }

    unsafe fn mix(&mut self, buffer: *mut AudioFrame, _rate_scale: f32, frames: i32) -> i32 {
        assert!(frames >= 0);

        // Don't block the audio thread if the mutex is in use
        let _guard = match self.mutex.try_lock() {
            Ok(guard) => guard,
            Err(WouldBlock {}) => return 0,
            Err(err @ Poisoned(_)) => panic!("poisoned mutex: {}", err),
        };

        // Don't mix audio if the JS context is invalid
        let mut js_context = match self.js_context.as_ref().map(RefCell::borrow_mut) {
            Some(js_context) => js_context,
            None => return 0,
        };

        let result = js_context.do_scoped(concat!(module_path!(), "::AudioStreamPlaybackBeepBox::mix"), &mut |scope| {
            let global = scope.get_current_context().global(scope);
            let audio_context = global.get(scope, "activeAudioContext").unwrap();
            if audio_context.is_undefined() {
                return Ok(());
            }
            let audio_context: v8::Local<v8::Object> = audio_context.try_into().unwrap();
            let script_processor: v8::Local<v8::Object> =
                audio_context.get(scope, "scriptProcessor")?.try_into()?;

            self.buffer_left.resize(frames as usize, 0.0);
            self.buffer_right.resize(frames as usize, 0.0);

            poll_js_audio(
                scope,
                frames as usize,
                audio_context,
                script_processor,
                &mut self.buffer_left,
                &mut self.buffer_right,
            )?;

            for i in 0..frames {
                buffer.offset(i as isize).write(AudioFrame { left: 0.0, right: 0.0 });
            }

            Ok(())
        });

        if let Err(error) = result {
            godot_error!("error in AudioStreamPlaybackBeepBox::mix: {error}");
        }

        frames
    }
}
