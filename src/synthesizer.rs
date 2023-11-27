use crate::js::{poll_audio, JSContext};
use crate::utils::V8ObjectHelpers;
use anyhow::Context;
use godot::engine::file_access::ModeFlags;
use godot::engine::{AudioStreamGenerator, AudioStreamGeneratorPlayback, FileAccess};
use godot::{engine::AudioStreamPlayer, prelude::*};
use std::fs::read_to_string;

const MAX_SAMPLES: usize = 1024 * 1024;
const SYNTH_INIT: &str = "synth = new beepbox.Synth()";

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
    fn import(&mut self, path: GString) {
        self.js
            .do_scoped("", |scope| {
                let path = path.to_string();
                let contents = if path.starts_with("res://") {
                    FileAccess::open(path.clone().into(), ModeFlags::READ)
                        .with_context(|| format!("failed to open {path}"))?
                        .get_as_text()
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
    fn eval(&mut self, code: GString) -> Variant {
        self.js.run("eval_bool", &code.to_string()).unwrap()
    }
}

#[godot_api]
impl IAudioStreamPlayer for Synthesizer {
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
                include_str!("../dependencies/jummbox/website/beepbox_synth.js"),
            )
            .unwrap();

        self.js
            .run("synth_init", SYNTH_INIT)
            .expect("failed to initialize synthethizer");

        self.base.play();
    }

    fn process(&mut self, _delta: f64) {
        if !self.base.has_stream_playback() {
            return;
        }

        let mut playback = self
            .base
            .get_stream_playback()
            .context("stream playback is missing")
            .unwrap()
            .cast::<AudioStreamGeneratorPlayback>();

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

    fn exit_tree(&mut self) {
        self.pause();
    }
}
