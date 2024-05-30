#[cfg(unix)]
fn exec_godot() {
    use std::env;
    use std::os::unix::process::CommandExt;
    use std::process::{exit, Command};

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

#[cfg(not(unix))]
fn exec_godot() {
    println!("This binary is only used to launch the Godot editor for debugging, using exec(), but unfortunately, this function is not supported in your platform.");
}

fn main() {
    exec_godot()
}
