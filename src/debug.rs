#![allow(unused)]

use std::{fs::{remove_file, File, OpenOptions}, io::Write, path::Path, process::{Child, Command}, sync::{Mutex, RwLock}, thread::sleep, time::Duration};
use clap::Parser;
use crossterm::terminal::{Clear, ClearType};
use lazy_static::lazy_static;
use chrono::Utc;

use crate::config;

lazy_static! {
    pub static ref DEBUGGER: RwLock<Option<Debugger>> = RwLock::new(
        if config::ARGS.debug {
            Some(Debugger::new())
        } else {
            None
        }
    );
}

pub struct Debugger {
    pub log: Vec<String>,
    pub log_file: File,
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {
            log: Vec::new(),
            log_file: OpenOptions::new()
                .append(true)
                .create(true)
                .open(format!("/home/alex/debug_{}.log", Utc::now().format("%Y-%m-%d")))
                .unwrap(),
        }
    }

    pub fn log(&mut self, msg: &str) {
        let msg = format!("{} {}", Utc::now().format("%H:%M:%S"), msg);
        self.log_file.write_all(format!("{}\n", msg).as_bytes());
        self.log.push(msg);
    }
}

#[macro_export]
macro_rules! debug_log {
    ($($args:tt)*) => {{
        if let Some(debugger) = &mut *crate::debug::DEBUGGER.write().unwrap() {
            debugger.log(&format!($($args)*))
        }
    }};
}

