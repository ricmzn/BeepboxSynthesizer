use std::{fs::read_to_string, time::Instant};

use anyhow::{anyhow, Context, Result};
use godot::engine::class_macros::auto_register_classes;
use godot::engine::file_access::ModeFlags;
use godot::engine::FileAccess;
use godot::{
    engine::{AudioStreamGenerator, AudioStreamGeneratorPlayback, AudioStreamPlayer},
    prelude::*,
};
use once_cell::sync::OnceCell;
use utils::V8ObjectHelpers;

mod utils;

const MAX_SAMPLES: usize = 1024 * 1024;
const SYNTH_INIT: &str = "synth = new beepbox.Synth()";

static REFERENCE_TIME: OnceCell<Instant> = OnceCell::new();

fn no_op(_: &mut v8::HandleScope, _: v8::FunctionCallbackArguments, _: v8::ReturnValue) {}

fn create_script_processor(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut ret: v8::ReturnValue,
) {
    let audio_context = args.this();
    let processor = v8::Object::new(scope);

    let sample_rate = v8::Number::new(scope, 44100.0);
    processor.set(scope, "sampleRate", sample_rate).unwrap();

    let no_op = v8::Function::new(scope, no_op).unwrap();
    processor.set(scope, "connect", no_op).unwrap();
    processor.set(scope, "disconnect", no_op).unwrap();

    audio_context
        .set(scope, "scriptProcessor", processor)
        .unwrap();

    let channel_length = MAX_SAMPLES * std::mem::size_of::<f32>();
    let array_buffer = v8::ArrayBuffer::new(scope, channel_length * 2);
    audio_context
        .set(scope, "outputArrayBuffer", array_buffer)
        .unwrap();

    let left_channel_buffer = v8::Float32Array::new(scope, array_buffer, 0, MAX_SAMPLES).unwrap();
    let right_channel_buffer =
        v8::Float32Array::new(scope, array_buffer, channel_length, MAX_SAMPLES).unwrap();

    let output_buffer = v8::Object::new(scope);

    let get_channel_data = v8::Function::new(scope, get_channel_data).unwrap();
    output_buffer
        .set(scope, "getChannelData", get_channel_data)
        .unwrap();

    output_buffer
        .set(scope, "leftChannelBuffer", left_channel_buffer)
        .unwrap();

    output_buffer
        .set(scope, "rightChannelBuffer", right_channel_buffer)
        .unwrap();

    audio_context
        .set(scope, "outputBuffer", output_buffer)
        .unwrap();

    ret.set(processor.into());
}

fn get_channel_data(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut ret: v8::ReturnValue,
) {
    let this = args.this();
    let index = args.get(0).int32_value(scope).unwrap();
    match index {
        0 => ret.set(this.get(scope, "leftChannelBuffer").unwrap()),
        1 => ret.set(this.get(scope, "rightChannelBuffer").unwrap()),
        _ => {}
    }
}

fn resume_playback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let audio_context = args.this();
    let bool = v8::Boolean::new(scope, true);
    audio_context.set(scope, "playing", bool).unwrap();
}

fn stop_playback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let audio_context = args.this();
    let bool = v8::Boolean::new(scope, false);
    audio_context.set(scope, "playing", bool).unwrap();
}

fn performance_now(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut ret: v8::ReturnValue,
) {
    let time_diff = Instant::now().duration_since(unsafe { *REFERENCE_TIME.get_unchecked() });
    let value = v8::Number::new(scope, time_diff.as_secs_f64() * 1000.0);
    ret.set(value.into());
}

fn new_audio_context(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut ret: v8::ReturnValue,
) {
    let audio_context = v8::Object::new(scope);
    let sample_rate = v8::Number::new(scope, 44100.0);
    audio_context.set(scope, "sampleRate", sample_rate).unwrap();

    let bool = v8::Boolean::new(scope, false);
    audio_context.set(scope, "playing", bool).unwrap();

    let global = scope.get_current_context().global(scope);
    global
        .set(scope, "activeAudioContext", audio_context)
        .unwrap();

    let create_script_processor = v8::Function::new(scope, create_script_processor).unwrap();
    audio_context
        .set(scope, "createScriptProcessor", create_script_processor)
        .unwrap();

    let resume_playback = v8::Function::new(scope, resume_playback).unwrap();
    audio_context.set(scope, "resume", resume_playback).unwrap();

    let stop_playback = v8::Function::new(scope, stop_playback).unwrap();
    audio_context.set(scope, "close", stop_playback).unwrap();

    ret.set(audio_context.into());
}

fn poll_audio(
    scope: &mut v8::HandleScope,
    required_samples: usize,
    audio_context: v8::Local<v8::Object>,
    script_processor: v8::Local<v8::Object>,
    buffer: &mut PackedVector2Array,
) -> Result<()> {
    let audio_process_callback: v8::Local<v8::Function> =
        script_processor.get(scope, "onaudioprocess")?.try_into()?;

    let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();

    let output_buffer: v8::Local<v8::Object> =
        audio_context.get(scope, "outputBuffer")?.try_into()?;

    let output_buffer_length = v8::Number::new(scope, required_samples as f64);
    output_buffer
        .set(scope, "length", output_buffer_length)
        .unwrap();

    let event = v8::Object::new(scope);
    event.set(scope, "outputBuffer", output_buffer)?;

    audio_process_callback.call(scope, undefined, &[event.into()]);

    let left_channel_buffer: v8::Local<v8::Float32Array> =
        output_buffer.get(scope, "leftChannelBuffer")?.try_into()?;
    let right_channel_buffer: v8::Local<v8::Float32Array> =
        output_buffer.get(scope, "rightChannelBuffer")?.try_into()?;

    // Transform JS audio output (individual channel streams) into Godot sound data (interleaved stereo stream)
    for i in 0..required_samples {
        buffer.set(
            i,
            Vector2::new(
                left_channel_buffer
                    .get_index(scope, i as u32)
                    .context("index out of bounds")?
                    .number_value(scope)
                    .context("value is not a number")? as f32,
                right_channel_buffer
                    .get_index(scope, i as u32)
                    .context("index out of bounds")?
                    .number_value(scope)
                    .context("value is not a number")? as f32,
            ),
        );
    }

    Ok(())
}

struct JSInspector {
    base: v8::inspector::V8InspectorClientBase,
}

impl v8::inspector::V8InspectorClientImpl for JSInspector {
    fn base(&self) -> &v8::inspector::V8InspectorClientBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut v8::inspector::V8InspectorClientBase {
        &mut self.base
    }

    fn console_api_message(
        &mut self,
        _context_group_id: i32,
        _level: i32,
        message: &v8::inspector::StringView,
        _url: &v8::inspector::StringView,
        _line_number: u32,
        _column_number: u32,
        _stack_trace: &mut v8::inspector::V8StackTrace,
    ) {
        godot_print!("{message}");
    }
}

struct JSContext {
    isolate: v8::OwnedIsolate,
    context: v8::Global<v8::Context>,
    inspector_client: JSInspector,
}

impl JSContext {
    fn new() -> Result<JSContext> {
        // Create the V8 sandbox
        let mut isolate = v8::Isolate::new(Default::default());
        let context = {
            // Create global variables and functions
            let mut scope = v8::HandleScope::new(&mut isolate);
            let global = v8::ObjectTemplate::new(&mut scope);

            global.set(
                v8::String::new(&mut scope, "AudioContext").unwrap().into(),
                v8::FunctionTemplate::new(&mut scope, new_audio_context).into(),
            );

            let performance = v8::ObjectTemplate::new(&mut scope);
            let performance_now = v8::FunctionTemplate::new(&mut scope, performance_now);
            performance.set(
                v8::String::new(&mut scope, "now").unwrap().into(),
                performance_now.into(),
            );
            global.set(
                v8::String::new(&mut scope, "performance").unwrap().into(),
                performance.into(),
            );

            global.set(
                v8::String::new(&mut scope, "oerformance").unwrap().into(),
                performance.into(),
            );

            let context = v8::Context::new_from_template(&mut scope, global);

            // Wrap the context in a global object so its lifetime is unbound
            v8::Global::new(&mut scope, context)
        };

        let mut context = JSContext {
            isolate,
            context,
            inspector_client: JSInspector {
                base: v8::inspector::V8InspectorClientBase::new::<JSInspector>(),
            },
        };

        // Bind the global object for compatibility with web browser scripts
        context.run("bootstrap", "const window = this")?;

        Ok(context)
    }

    fn run(&mut self, filename: &str, src: &str) -> Result<Variant> {
        self.do_scoped(filename, |scope| {
            // Build and run the script
            let src = v8::String::new(scope, src).context("could not build v8 string")?;
            let value = v8::Script::compile(scope, src, None)
                .context("failed to compile script")?
                .run(scope)
                .context("missing return value")?;
            Ok(utils::v8_value_to_godot_variant(scope, value))
        })
    }

    // FIXME: Workaround for https://github.com/godot-rust/gdext/issues/159
    fn run_bool(&mut self, filename: &str, src: &str) -> Result<bool> {
        self.do_scoped(filename, |scope| {
            // Build and run the script
            let src = v8::String::new(scope, src).context("could not build v8 string")?;
            let value = v8::Script::compile(scope, src, None)
                .context("failed to compile script")?
                .run(scope)
                .context("missing return value")?;
            Ok(value.boolean_value(scope))
        })
    }

    // FIXME: Workaround for https://github.com/godot-rust/gdext/issues/159
    fn run_int(&mut self, filename: &str, src: &str) -> Result<i64> {
        self.do_scoped(filename, |scope| {
            // Build and run the script
            let src = v8::String::new(scope, src).context("could not build v8 string")?;
            let value = v8::Script::compile(scope, src, None)
                .context("failed to compile script")?
                .run(scope)
                .context("missing return value")?;
            value
                .integer_value(scope)
                .ok_or(anyhow!("value is not a number"))
        })
    }

    // FIXME: Workaround for https://github.com/godot-rust/gdext/issues/159
    fn run_string(&mut self, filename: &str, src: &str) -> Result<String> {
        self.do_scoped(filename, |scope| {
            // Build and run the script
            let src = v8::String::new(scope, src).context("could not build v8 string")?;
            let value = v8::Script::compile(scope, src, None)
                .context("failed to compile script")?
                .run(scope)
                .context("missing return value")?;
            Ok(value.to_rust_string_lossy(scope))
        })
    }

    fn do_scoped<'scope, T>(
        &'scope mut self,
        filename: &str,
        mut callback: impl FnMut(&mut v8::HandleScope<'scope>) -> Result<T>,
    ) -> Result<T> {
        // "Raw" script scope
        let mut scope = v8::HandleScope::new(&mut self.isolate);
        let context = v8::Local::new(&mut scope, self.context.clone());

        // Create and bind an inspector for console logging
        let mut inspector =
            v8::inspector::V8Inspector::create(&mut scope, &mut self.inspector_client);
        inspector.context_created(
            context,
            1,
            v8::inspector::StringView::from(b"Inspector" as &[u8]),
        );

        // Script scope with globals + error handling
        let mut scope = v8::ContextScope::new(&mut scope, context);
        let mut scope = v8::TryCatch::new(&mut scope);

        // Run user callback using the scope
        let script_result = callback(&mut scope);

        if scope.has_caught() {
            let message = scope.message().context("could not extract error message")?;
            return script_result.context(anyhow!(
                "{} ({filename}:{})",
                message.get(&mut scope).to_rust_string_lossy(&mut scope),
                message.get_line_number(&mut scope).unwrap_or(0),
            ));
        }

        script_result
    }
}

#[derive(GodotClass)]
#[class(base = AudioStreamPlayer)]
pub struct Synthesizer {
    #[base]
    base: Base<AudioStreamPlayer>,
    buffer: PackedVector2Array,
    js: JSContext,
}

#[godot_api]
impl Synthesizer {
    #[func]
    fn resume(&mut self) {
        self.js
            .run("resume", "synth.resetEffects(); synth.play()")
            .unwrap();
    }

    #[func]
    fn pause(&mut self) {
        self.js.run("pause", "synth.pause()").unwrap();
    }

    #[func]
    fn import(&mut self, path: GodotString) {
        self.js
            .do_scoped("", |scope| {
                let path = path.to_string();
                let contents = if path.starts_with("res://") {
                    FileAccess::open(path.clone().into(), ModeFlags::READ)
                        .with_context(|| format!("failed to open {path}"))?
                        .get_as_text(true)
                        .to_string()
                } else {
                    read_to_string(&path).with_context(|| format!("could not read {path}"))?
                };

                let json_string =
                    v8::String::new(scope, &contents).context("could not build v8 string")?;

                let synth: v8::Local<v8::Object> = scope
                    .get_current_context()
                    .global(scope)
                    .get(scope, "synth")?
                    .try_into()?;

                let set_song: v8::Local<v8::Function> = synth
                    .get(scope, "setSong")?
                    .try_into()
                    .context("synth.setSong not defined")?;

                set_song.call(scope, synth.into(), &[json_string.into()]);
                Ok(())
            })
            .unwrap();
    }

    #[func]
    fn eval(&mut self, code: GodotString) -> Variant {
        // FIXME: Workaround for https://github.com/godot-rust/gdext/issues/195
        let code = std::mem::ManuallyDrop::new(code);
        self.js.run("eval_bool", &code.to_string()).unwrap()
    }

    #[func]
    fn eval_bool(&mut self, code: GodotString) -> bool {
        // FIXME: Workaround for https://github.com/godot-rust/gdext/issues/195
        let code = std::mem::ManuallyDrop::new(code);
        self.js.run_bool("eval_bool", &code.to_string()).unwrap()
    }

    #[func]
    fn eval_int(&mut self, code: GodotString) -> i64 {
        // FIXME: Workaround for https://github.com/godot-rust/gdext/issues/195
        let code = std::mem::ManuallyDrop::new(code);
        self.js.run_int("eval_int", &code.to_string()).unwrap()
    }

    #[func]
    fn eval_string(&mut self, code: GodotString) -> GodotString {
        // FIXME: Workaround for https://github.com/godot-rust/gdext/issues/195
        let code = std::mem::ManuallyDrop::new(code);
        let rust_string = self
            .js
            .run_string("eval_string", &code.to_string())
            .unwrap();
        GodotString::from(rust_string)
    }
}

#[godot_api]
impl AudioStreamPlayerVirtual for Synthesizer {
    fn init(mut base: Base<AudioStreamPlayer>) -> Self {
        // Set up Godot audio player
        let mut generator = AudioStreamGenerator::new();
        generator.set_mix_rate(44100.0);
        generator.set_buffer_length(0.1);
        base.set_stream(generator.upcast());

        let mut buffer = PackedVector2Array::new();
        buffer.resize(MAX_SAMPLES);

        Synthesizer {
            base,
            buffer,
            js: JSContext::new().expect("failed to create js context"),
        }
    }

    fn ready(&mut self) {
        self.js
            .run(
                "beepbox_synth.js",
                include_str!("../dependencies/beepbox/website/beepbox_synth.js"),
            )
            .unwrap();

        self.js
            .run("synth_init", SYNTH_INIT)
            .expect("failed to initialize synthethizer");

        self.base.play(0.0);
    }

    fn process(&mut self, _delta: f64) {
        if !self.has_stream_playback() {
            return;
        }

        let mut playback = self
            .get_stream_playback()
            .context("stream playback is missing")
            .unwrap()
            .try_cast::<AudioStreamGeneratorPlayback>()
            .context("stream playback is not an instance of AudioStreamGeneratorPlayback")
            .unwrap();

        let result = self.js.do_scoped("_process", |scope| {
            let global = scope.get_current_context().global(scope);
            let audio_context = global.get(scope, "activeAudioContext").unwrap();
            if audio_context.is_undefined() {
                return Ok(());
            }
            let audio_context: v8::Local<v8::Object> = audio_context.try_into().unwrap();
            let script_processor: v8::Local<v8::Object> =
                audio_context.get(scope, "scriptProcessor")?.try_into()?;

            // Ask godot how much data needs to be filled
            let required_samples = playback.get_frames_available() as usize;

            // Pull data from Beepbox
            poll_audio(
                scope,
                required_samples,
                audio_context,
                script_processor,
                &mut self.buffer,
            )?;

            // Fill Godot's sound buffer
            for i in 0..required_samples {
                playback.push_frame(self.buffer.get(i));
            }

            Ok(())
        });

        if let Err(error) = result {
            godot_error!("error in Synthesizer::process: {error}");
        }
    }
}

fn init_v8() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
}

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
        init_v8();
        true
    }
}
