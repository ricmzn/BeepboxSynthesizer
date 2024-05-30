use std::fs::read_to_string;
use std::io;

use anyhow::{anyhow, Context};
use godot::engine::{
    file_access::ModeFlags, AudioStreamGenerator, AudioStreamGeneratorPlayback, FileAccess,
    IAudioStreamPlayer,
};
use godot::{engine::AudioStreamPlayer, prelude::*};
use rquickjs::{function::This, Function, Object, Value};
use rquickjs::prelude::Rest;

use crate::js::{poll_audio, JSContext};
use crate::js_console::console_log;

const MAX_SAMPLES: usize = 1024 * 1024;
const SYNTH_INIT: &str = "synth = new beepbox.Synth()";

#[derive(GodotClass)]
#[class(base = AudioStreamPlayer)]
pub struct Synthesizer {
    base: Base<AudioStreamPlayer>,
    audio_buffer: PackedVector2Array,
    js_context: JSContext,
}

#[godot_api]
impl Synthesizer {
    #[func]
    fn resume(&mut self) {
        self.js_context
            .run("resume", "synth.resetEffects(); synth.play()")
            .unwrap();
    }

    #[func]
    fn pause(&mut self) {
        self.js_context.run("pause", "synth.pause()").unwrap();
    }

    #[func]
    fn import(&mut self, path: GString) {
        self.js_context
            .with_context(|ctx| {
                let path = path.to_string();
                let contents = if path.starts_with("res://") {
                    FileAccess::open(path.clone().into(), ModeFlags::READ)
                        .ok_or_else(|| io_error(io::ErrorKind::NotFound, &path))?
                        .get_as_text()
                        .to_string()
                } else {
                    read_to_string(&path).map_err(|err| io_error(err.kind(), &path))?
                };

                let song_json = ctx.json_parse(contents)?;

                let synth: Object = ctx.globals().get("synth")?;

                let set_song: Function = synth.get("setSong")?;

                set_song.call((This(synth.clone()), song_json))?;

                let song: Option<Object> = synth.get("song")?;

                Ok(())
            })
            .unwrap();
    }

    #[func]
    fn eval(&mut self, code: GString) -> Variant {
        self.js_context.run("eval", &code.to_string()).unwrap()
    }
}

#[godot_api]
impl IAudioStreamPlayer for Synthesizer {
    fn init(base: Base<AudioStreamPlayer>) -> Self {
        let mut generator = AudioStreamGenerator::new_gd();
        generator.set_mix_rate(44100.0);
        generator.set_buffer_length(1.0);

        let mut audio_buffer = PackedVector2Array::new();
        audio_buffer.resize(MAX_SAMPLES);

        base.to_gd().set_stream(generator.upcast());

        Self {
            base,
            audio_buffer,
            js_context: JSContext::new().expect("failed to create js context"),
        }
    }

    fn process(&mut self, _delta: f64) {
        if !self.base_mut().has_stream_playback() {
            return;
        }

        let mut playback = self
            .base_mut()
            .get_stream_playback()
            .context("stream playback is missing")
            .unwrap()
            .cast::<AudioStreamGeneratorPlayback>();

        let result = self.js_context.with_context(|ctx| {
            let audio_context: Value = ctx.globals().get("activeAudioContext")?;
            if audio_context.is_undefined() {
                return Ok(());
            }

            let audio_context = audio_context.as_object().unwrap();

            let script_processor: Object = audio_context.get("scriptProcessor")?;

            // Ask godot how much data needs to be filled
            let required_samples = playback.get_frames_available() as usize;

            // Pull data from Beepbox
            poll_audio(
                ctx,
                required_samples,
                &audio_context,
                &script_processor,
                &mut self.audio_buffer,
            )?;

            // Fill Godot's sound buffer
            for i in 0..required_samples {
                playback.push_frame(self.audio_buffer.get(i).unwrap());
            }

            Ok(())
        });

        if let Err(error) = result {
            godot_error!("error in Synthesizer::process: {error}");
        }
    }

    fn exit_tree(&mut self) {
        self.pause();
    }

    fn ready(&mut self) {
        self.js_context
            .run(
                "beepbox_synth.js",
                include_str!("../dependencies/jummbox/website/beepbox_synth.js"),
            )
            .expect("failed to run beepbox script");

        self.js_context
            .run("synth_init", SYNTH_INIT)
            .expect("failed to initialize beepbox synthethizer");

        self.base_mut().play();
    }
}

fn io_error(kind: io::ErrorKind, path: &str) -> rquickjs::Error {
    rquickjs::Error::Io(io::Error::new(kind, anyhow!("failed to open {}", path)))
}
