#![allow(unused)]

use std::{fs::remove_file, process::{Command, Child}, thread::sleep, time::Duration};
use crossterm::terminal::{Clear, ClearType};

use crate::config;

// pub const DEBUG_PATH: &str = "/tmp/tetris_debug_pipe";

#[macro_export]
macro_rules! debug_println {
    ($($args:tt)*) => {{
        use std::{io::Write, fs::OpenOptions};

        let path = format!("/tmp/tetris_debug_pipe{}", *config::BIND_ADDR);

        let mut pipe = OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap_or_else(|_| panic!("failed to open {}", &path));

        writeln!(pipe, $($args)*).unwrap();
        pipe.flush().unwrap();
    }};
}

pub struct DebugWindow {
    child: Child,
    path: String,
}

impl DebugWindow {
    pub fn new() -> DebugWindow {
        let path = format!("/tmp/tetris_debug_pipe{}", *config::BIND_ADDR);

        remove_file(&path).ok();

        Command::new("mkfifo").arg(&path).output().unwrap();

        let child = Command::new("kitty")
            .arg("--title")
            .arg("TETRIS - Debug")
            .arg("bash")
            .arg("-c")
            .arg(format!("tail -f {}", &path))
            .spawn()
            .unwrap();

        sleep(Duration::from_secs(1));

        DebugWindow { child, path }
    }

    pub fn close(mut self) {
        remove_file(self.path).ok();

        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }
}

