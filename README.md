# Introduction

This project uses a custom GDExtension library written in Rust to run the JummBox JavaScript-based synthesizer within Godot 4 with [V8](https://v8.dev/), the JavaScript engine used in Node.js.

The goals are to allow use of smaller music files (using procedural playback instead of rendered WAV or MP3 files), and experimenting with with dynamic music playback (using JummBox's JavaScript APIs).

## References

* Rust GDExtension Library for Godot: https://github.com/godot-rust/gdext
* BeepBox: https://github.com/johnnesky/beepbox (Upstream code for JummBox)
* JummBox: https://github.com/jummbus/jummbox (Backward-compatible fork of BeepBox)
* Rust V8 Library: https://github.com/denoland/rusty_v8

# Building

## Required Tools

* Godot 4.2+: https://godotengine.org/download 
* Rust 1.78+: https://www.rust-lang.org/tools/install
* Node.js 20+: https://nodejs.org

## Instructions

* Clone this repository with the `--recursive` flag to include submodules
  - Git submodules manual: https://git-scm.com/book/en/v2/Git-Tools-Submodules
* Build JummBox
  - Enter the `dependencies/jummbox` directory and run: `npm install && npm update && npm run build-synth`
* Build this project by running `cargo build` in the root of the repository
* Open this project in Godot
