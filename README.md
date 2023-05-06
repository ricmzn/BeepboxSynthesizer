# Introduction

This project uses a custom Rust-based GDExtension library to run the BeepBox/JummBox synthesizer within Godot 4, using V8.

The primary goal is to allow use of smaller music files, as well as experimentation with procedural music (using BeepBox's JavaScript API).

* Rust GDExtension: https://github.com/godot-rust/gdext
* BeepBox: https://github.com/johnnesky/beepbox
* JummBox: https://github.com/jummbus/jummbox (Backward-compatible fork of BeepBox)
* V8: https://github.com/denoland/rusty_v8

> Note: because [AudioStreamGenerator is currently broken in Godot 4.0.2](https://github.com/godotengine/godot/issues/65155), a custom version of the engine needs to be built with the following patch: https://github.com/godotengine/godot/pull/73162/files, or generated audio will not play.

> Note: the project is currently only tested on Linux, but it should work on Windows if the DLL paths are added to `beepbox_synthesizer.gdextension`

# Building

You will need:

* Rust: https://www.rust-lang.org/tools/install
* Node.js: https://nodejs.org

Instructions:

* Build the latest Godot 4 stable release with the AudioStreamGenerator patch added (not required if already fixed upstream)
* Clone this repository with the `--recursive` flag so submodules are also fetched (more about them: https://git-scm.com/book/en/v2/Git-Tools-Submodules)
* Build the JummBox synthesizer
  - Enter `dependencies/jummbox` and run `npm install && npm update && npm run build-synth`
  - `npm update` is only required on NodeJS v18 or later due to some depdendencies being incompatible unless manually updated
* Build the Rust crate .dll/.so by running `cargo build` in the root of the project
* Run this project with the custom Godot 4 editor build

# Troubleshooting

* The editor crashes with SIGILL (Illegal Instruction) on startup!
  - I haven't look into why this happens the first time after setting up the project from a clean state, but usually, building the .dll/.so a second time lets the editor launch correctly.
* I load a JSON, click play, but there is no sound!
  - Make sure you have built and ran Godot with the AudioStreamGenerator patch. The current official release (4.0.2) has no issues playing static audio files, but it will not play script-generated audio until the feature is fixed.
