#![allow(unused)]

use std::{fs::remove_file, path::Path, process::{Child, Command}, thread::sleep, time::Duration};
use crossterm::terminal::{Clear, ClearType};

use crate::config;

pub const DEBUG_PATH: &str = "/tmp/tetris_debug_pipe";

#[macro_export]
macro_rules! debug_println {
    ($($args:tt)*) => {{
        use std::{io::Write, fs::OpenOptions};

        let mut pipe = OpenOptions::new()
            .write(true)
            .open(crate::debug::DEBUG_PATH)
            .unwrap_or_else(|_| panic!("failed to open {}", crate::debug::DEBUG_PATH));

        writeln!(pipe, "{}", format!($($args)*)).unwrap();
        pipe.flush().unwrap();
    }};
}

pub struct DebugWindow {
    child: Option<Child>,
}

impl DebugWindow {
    pub fn new() -> DebugWindow {
        let child = if !Path::new(DEBUG_PATH).exists() {
            Command::new("mkfifo").arg(DEBUG_PATH).output().unwrap();
            Some(Command::new("kitty")
                .arg("--title")
                .arg("TETRIS - Debug")
                .arg("bash")
                .arg("-c")
                .arg(format!("tail -f {}", DEBUG_PATH))
                .spawn()
                .unwrap())
        } else {
            None
        };

        sleep(Duration::from_secs(1));

        DebugWindow { child }
    }

    pub fn close(mut self) {
        remove_file(DEBUG_PATH).ok();

        if let Some(mut child) = self.child {
            child.kill().unwrap();
            child.wait().unwrap();
        }
    }
}

