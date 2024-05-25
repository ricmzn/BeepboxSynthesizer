use std::env;
use std::os::unix::process::CommandExt;
use std::process::{Command, exit};

fn main() {
    match env::var("GODOT_BIN") {
        Ok(godot_bin) => {
            let error = Command::new(&godot_bin).args(["--path", "."]).exec();
            panic!("{}: {}", godot_bin, error);
        }
        Err(err) => {
            eprintln!("{err}: GODOT_BIN");
            exit(1);
        }
    }
}
