use std::cell::RefCell;
use std::fs::read_to_string;

use anyhow::{Context, Result};
use godot::engine::{AudioStream, AudioStreamPlayback, FileAccess, IAudioStream};
use godot::engine::file_access::ModeFlags;
use godot::prelude::*;

use crate::classes::audio_stream_playback_beep_box::AudioStreamPlaybackBeepBox;
use crate::js_bridge::util::V8ObjectHelpers;

#[derive(GodotClass)]
#[class(base = AudioStream)]
pub struct AudioStreamBeepBox {
    base: Base<AudioStream>,
    playbacks: RefCell<Vec<Gd<AudioStreamPlaybackBeepBox>>>,
}

#[godot_api]
impl AudioStreamBeepBox {
    #[func]
    pub fn import(&mut self, path: GString) {
        self.do_scoped_in_playbacks(concat!(module_path!(), "AudioStreamBeepBox::import"), &mut |scope| {
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
                .context("synth.setSong is not defined")?;

            set_song.call(scope, synth.into(), &[json_string.into()]);

            Ok(Variant::nil())
        })
    }

    fn do_scoped_in_playbacks(&self, filename: &str, callback: &mut dyn FnMut(&mut v8::HandleScope<'_>) -> Result<Variant>) {
        for playback in self.playbacks.borrow().iter() {
            playback.bind().do_scoped_with_mutex(filename, callback).inspect_err(|err| {
                godot_error!("{filename} failed, reason: {err}");
            }).ok();
        }
    }
}

#[godot_api]
impl IAudioStream for AudioStreamBeepBox {
    fn init(base: Base<AudioStream>) -> Self {
        godot_print!("AudioStreamBeepBox.init()");
        AudioStreamBeepBox {
            base,
            playbacks: RefCell::new(Vec::new()),
        }
    }

    fn instantiate_playback(&self) -> Option<Gd<AudioStreamPlayback>> {
        godot_print!("AudioStreamBeepBox.instantiate_playback()");
        let playback = Gd::from_init_fn(AudioStreamPlaybackBeepBox::new);
        self.playbacks.borrow_mut().push(playback.to_godot());
        Some(playback.upcast())
    }
}
