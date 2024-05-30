use std::time::Instant;

use anyhow::{anyhow, Context as _};
use godot::prelude::*;
use rquickjs::context::EvalOptions;
use rquickjs::prelude::{FuncArg, This};
use rquickjs::{ArrayBuffer, Context, Ctx, Function, Object, Result, Runtime, Undefined, Value};

use crate::js_console::{console_error, console_log, console_warn};
use crate::{utils::js_value_to_godot_variant, REFERENCE_TIME};

const MAX_SAMPLES: usize = 1024 * 1024;

fn no_op() {}

fn create_script_processor<'a>(
    ctx: Ctx<'a>,
    audio_context: This<Object<'a>>,
) -> Result<Object<'a>> {
    let processor = Object::new(ctx.clone())?;

    let sample_rate = 44100.0;
    processor.set("sampleRate", sample_rate)?;
    processor.set("connect", Function::new(ctx.clone(), no_op))?;
    processor.set("disconnect", Function::new(ctx.clone(), no_op))?;

    audio_context.set("scriptProcessor", processor.clone())?;

    let channel_length = MAX_SAMPLES * std::mem::size_of::<f32>();
    let array_buffer = ArrayBuffer::new(ctx.clone(), vec![0f32; channel_length])?;

    audio_context.set("outputArrayBuffer", array_buffer)?;

    let left_channel_buffer = vec![0f32; MAX_SAMPLES];
    let right_channel_buffer = vec![0f32; MAX_SAMPLES];
    let output_buffer = Object::new(ctx.clone())?;

    output_buffer.set(
        "getChannelData",
        Function::new(ctx.clone(), get_channel_data),
    )?;

    output_buffer.set("leftChannelBuffer", left_channel_buffer)?;
    output_buffer.set("rightChannelBuffer", right_channel_buffer)?;
    audio_context.set("outputBuffer", output_buffer)?;

    Ok(processor)
}

fn get_channel_data<'a>(
    ctx: Ctx<'a>,
    output_buffer: This<Object<'a>>,
    index: FuncArg<f64>,
) -> Value<'a> {
    let undefined = Value::new_undefined(ctx);
    match *index as i32 {
        0 => output_buffer.get("leftChannelBuffer").unwrap_or(undefined),
        1 => output_buffer.get("rightChannelBuffer").unwrap_or(undefined),
        _ => undefined,
    }
}

fn resume_playback(_: Ctx, audio_context: This<Object>) -> Result<()> {
    audio_context.set("playing", true)
}

fn stop_playback(_: Ctx, audio_context: This<Object>) -> Result<()> {
    audio_context.set("playing", false)
}

fn performance_now() -> f64 {
    let reference_time = unsafe { *REFERENCE_TIME.get_unchecked() };
    let time_difference = Instant::now().duration_since(reference_time);
    time_difference.as_secs_f64() * 1000.0
}

fn new_audio_context(ctx: Ctx) -> Result<Object> {
    let audio_context = Object::new(ctx.clone())?;

    audio_context.set("sampleRate", 44100.0)?;
    audio_context.set("playing", false)?;
    audio_context.set(
        "createScriptProcessor",
        Function::new(ctx.clone(), create_script_processor),
    )?;
    audio_context.set("resume", Function::new(ctx.clone(), resume_playback))?;
    audio_context.set("close", Function::new(ctx.clone(), stop_playback))?;

    ctx.globals()
        .set("activeAudioContext", audio_context.clone())?;

    Ok(audio_context)
}

pub fn poll_audio<'a>(
    ctx: Ctx<'a>,
    required_samples: usize,
    audio_context: &Object<'a>,
    script_processor: &Object<'a>,
    buffer: &mut PackedVector2Array,
) -> Result<()> {
    let audio_process_callback: Function = script_processor.get("onaudioprocess")?;

    let output_buffer: Object = audio_context.get("outputBuffer")?;
    output_buffer.set("length", required_samples as f64)?;

    let event = Object::new(ctx.clone())?;
    event.set("outputBuffer", output_buffer.clone())?;

    audio_process_callback.call((Undefined, vec![event]))?;

    let left_channel_buffer: Vec<f32> = output_buffer.get("leftChannelBuffer")?;
    let right_channel_buffer: Vec<f32> = output_buffer.get("rightChannelBuffer")?;

    // Transform JS audio output (individual channel streams) into Godot sound data (interleaved stereo stream)
    for i in 0..required_samples {
        buffer[i] = Vector2::new(left_channel_buffer[i], right_channel_buffer[i]);
    }

    Ok(())
}

pub struct JSContext {
    #[allow(unused)]
    runtime: Runtime,
    context: Context,
}

impl JSContext {
    pub fn new() -> anyhow::Result<JSContext> {
        let runtime = Runtime::new().context("failed to create javascript runtime")?;
        let context = Context::full(&runtime).context("failed to create javascript context")?;

        // Create global variables and functions
        context.with::<_, anyhow::Result<()>>(|ctx| {
            ctx.globals().set("global", ctx.globals())?;
            ctx.globals().set("window", ctx.globals())?;

            ctx.globals().set(
                "AudioContext",
                Function::new(ctx.clone(), new_audio_context),
            )?;
            ctx.globals().set("navigator", Object::new(ctx.clone()))?;
            ctx.globals().set("document", Object::new(ctx.clone()))?;

            let console = Object::new(ctx.clone())?;
            console.set("log", Function::new(ctx.clone(), console_log))?;
            console.set("info", Function::new(ctx.clone(), console_log))?;
            console.set("debug", Function::new(ctx.clone(), console_log))?;
            console.set("trace", Function::new(ctx.clone(), console_log))?;
            console.set("warn", Function::new(ctx.clone(), console_warn))?;
            console.set("error", Function::new(ctx.clone(), console_error))?;
            ctx.globals().set("console", console)?;

            let performance = Object::new(ctx.clone())?;
            performance.set("now", Function::new(ctx.clone(), performance_now))?;
            ctx.globals().set("performance", performance)?;

            Ok(())
        })?;

        Ok(JSContext { runtime, context })
    }

    pub fn run(&self, filename: &str, src: &str) -> anyhow::Result<Variant> {
        let mut options = EvalOptions::default();
        options.strict = false;
        self.context
            .with(|ctx| match ctx.eval_with_options(src, options) {
                Ok(value) => Ok(js_value_to_godot_variant(value)),
                Err(err) => {
                    if let Some(js_err) = ctx.catch().as_exception() {
                        let message = js_err.message().unwrap_or_default();
                        let stack = js_err
                            .stack()
                            .unwrap_or_default()
                            .replace("eval_script", filename);
                        Err(anyhow!("{message}\n{stack}"))
                    } else {
                        Err(err).context("internal error")
                    }
                }
            })
    }

    pub fn with_context<T>(&self, callback: impl FnMut(Ctx) -> Result<T>) -> Result<T> {
        self.context.with(callback)
    }
}
