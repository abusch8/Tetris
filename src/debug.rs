#![allow(unused)]

use std::{fs::remove_file, process::{Command, Child}, thread::sleep, time::Duration};

pub const DEBUG_PATH: &str = "/tmp/tetris_debug_pipe";

#[macro_export]
macro_rules! debug_println {
    ($($args:tt)*) => {
        let mut pipe = std::fs::OpenOptions::new()
            .write(true)
            .open(DEBUG_PATH)
            .unwrap_or_else(|_| panic!("failed to open {}", DEBUG_PATH));
        writeln!(pipe, $($args)*).unwrap();
        pipe.flush().unwrap();
    };
}

pub struct DebugWindow {
    child: Child,
}

impl DebugWindow {
    pub fn new() -> DebugWindow {
        remove_file(DEBUG_PATH).ok();

        Command::new("mkfifo").arg(DEBUG_PATH).output().unwrap();

        let child = Command::new("kitty")
            .arg("--title")
            .arg("TETRIS - Debug")
            .arg("bash")
            .arg("-c")
            .arg(format!("tail -f {}", DEBUG_PATH))
            .spawn()
            .unwrap();

        sleep(Duration::from_secs(1));

        DebugWindow { child }
    }

    pub fn close(mut self) {
        remove_file(DEBUG_PATH).ok();

        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }
}
