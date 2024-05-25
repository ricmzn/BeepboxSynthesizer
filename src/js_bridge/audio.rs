use anyhow::{Context, Result};

use crate::js_bridge::util::V8ObjectHelpers;

// Each sample is one f32 so the total buffer size is 16 MiB
pub const MAX_SAMPLES: usize = 4 * 1024 * 1024;

fn no_op(_: &mut v8::HandleScope, _: v8::FunctionCallbackArguments, _: v8::ReturnValue) {}

fn create_script_processor(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut return_value: v8::ReturnValue,
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

    return_value.set(processor.into());
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

pub fn poll_js_audio(
    scope: &mut v8::HandleScope,
    required_samples: usize,
    audio_context: v8::Local<v8::Object>,
    script_processor: v8::Local<v8::Object>,
    buffer_left: &mut Vec<f32>,
    buffer_right: &mut Vec<f32>,
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

    for i in 0..required_samples {
        buffer_left[i] = left_channel_buffer
            .get_index(scope, i as u32)
            .context("index out of bounds")?
            .number_value(scope)
            .context("value is not a number")? as f32;

        buffer_right[i] = right_channel_buffer
            .get_index(scope, i as u32)
            .context("index out of bounds")?
            .number_value(scope)
            .context("value is not a number")? as f32;
    }

    Ok(())
}

pub fn create_audio_context(
    scope: &mut v8::HandleScope,
    _: v8::FunctionCallbackArguments,
    mut return_value: v8::ReturnValue,
) {
    let audio_context = v8::Object::new(scope);
    let sample_rate = v8::Number::new(scope, 44100.0);
    audio_context.set(scope, "sampleRate", sample_rate).unwrap();

    let bool = v8::Boolean::new(scope, false);
    audio_context.set(scope, "playing", bool).unwrap();

    scope.get_current_context().global(scope)
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

    return_value.set(audio_context.into());
}
