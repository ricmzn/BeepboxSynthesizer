This project uses a custom Rust-based GDExtension library to run the Beepbox synthesizer within Godot 4, using V8.

The primary goal is to allow use of smaller music files, as well as experimentation with procedural music (using Beepbox's JavaScript API).

* Rust GDExtension: https://github.com/godot-rust/gdext
* Beepbox: https://github.com/johnnesky/beepbox
* V8: https://github.com/denoland/rusty_v8

> Note: because [AudioStreamPlayer is currently broken in Godot 4.0.2](https://github.com/godotengine/godot/issues/65155), a custom version of the engine needs to be built with the following patch: https://github.com/godotengine/godot/pull/73162/files, or generated audio will not play.
