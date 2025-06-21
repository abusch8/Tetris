use std::net::SocketAddr;
use std::env;

use clap::Parser;
use ini::Ini;
use home::home_dir;
use lazy_static::lazy_static;

use crate::Cli;

lazy_static! {
    pub static ref CLI: Cli = Cli::parse();

    static ref CONFIG_PATH: String = env::var("TETRIS_CONFIG_PATH").unwrap_or(format!("{}/.config/tetris.ini", home_dir().unwrap().to_str().unwrap()));
    static ref CONFIG: Ini = Ini::load_from_file(&*CONFIG_PATH).unwrap_or(Ini::new());

    pub static ref MAX_FRAME_RATE: u64 = CONFIG
        .get_from_or(Some("display"), "max_frame_rate", "60")
        .parse()
        .unwrap_or_else(|_| panic!("Invalid max_frame_rate display config value"));

    pub static ref USE_XTERM_256_COLORS: bool = CONFIG
        .get_from_or(Some("display"), "use_xterm_256_colors", "true")
        .parse()
        .unwrap_or_else(|_| panic!("Invalid use_xterm_256_colors display config value"));

    pub static ref DISPLAY_FRAME_RATE: bool = CONFIG
        .get_from_or(Some("display"), "display_frame_rate", "false")
        .parse()
        .unwrap_or_else(|_| panic!("Invalid display_frame_rate display config value"));

    pub static ref DISPLAY_PING: bool = CONFIG
        .get_from_or(Some("display"), "display_ping", "true")
        .parse()
        .unwrap_or_else(|_| panic!("Invalid display_ping display config value"));

    pub static ref PARTY_MODE: bool = CLI.party || CONFIG
        .get_from_or(Some("display"), "party_mode", "false")
        .parse()
        .unwrap_or_else(|_| panic!("Invalid party_mode display config value"));

    pub static ref BIND_ADDR: SocketAddr = match &CLI.bind_addr {
        Some(addr) => addr,
        None => CONFIG.get_from_or(Some("multiplayer"), "bind_addr", "0.0.0.0:12000"),
    }
    .parse::<SocketAddr>()
    .unwrap_or_else(|_| panic!("Invalid bind_addr format"));

    pub static ref CONN_ADDR: SocketAddr = match &CLI.conn_addr {
        Some(addr) => addr,
        None => CONFIG.get_from_or(Some("multiplayer"), "conn_addr", "0.0.0.0:12000"),
    }
    .parse::<SocketAddr>()
    .unwrap_or_else(|_| panic!("Invalid conn_addr format"));
}

pub mod controls {

    use std::collections::HashMap;
    use crossterm::event::KeyCode;
    use lazy_static::lazy_static;

    use crate::{config::CONFIG, event::InputAction};

    fn key_map(key: &str, action: InputAction) -> HashMap<KeyCode, InputAction> {
        let mut action_map = HashMap::new();
        match key {
            key if key.len() == 1 && key.is_ascii() => {
                let char = key.chars().next().unwrap();
                action_map.insert(KeyCode::Char(char.to_ascii_uppercase()), action.clone());
                action_map.insert(KeyCode::Char(char.to_ascii_lowercase()), action);
            }
            "up" => {
                action_map.insert(KeyCode::Up, action);
            },
            "down" => {
                action_map.insert(KeyCode::Down, action);
            },
            "left" => {
                action_map.insert(KeyCode::Left, action);
            },
            "right" => {
                action_map.insert(KeyCode::Right, action);
            },
            "space" => {
                action_map.insert(KeyCode::Char(' '), action);
            },
            "escape" => {
                action_map.insert(KeyCode::Esc, action);
            },
            _ => panic!("Invalid controls config key value: {}", key),
        }
        action_map
    }

    lazy_static! {
        pub static ref ACTION_MAP: HashMap<KeyCode, InputAction> = {
            let mut action_map = HashMap::new();

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "move_right", "right, d")
                .to_string()
                .split(',')
                .map(str::trim)
                .flat_map(|key| key_map(key, InputAction::MoveRight)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "move_left", "left, a")
                .to_string()
                .split(',')
                .map(str::trim)
                .flat_map(|key| key_map(key, InputAction::MoveLeft)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "rotate_right", "up, w")
                .to_string()
                .split(',')
                .map(str::trim)
                .flat_map(|key| key_map(key, InputAction::RotateRight)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "rotate_left", "z")
                .to_string()
                .split(',')
                .map(str::trim)
                .flat_map(|key| key_map(key, InputAction::RotateLeft)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "soft_drop", "down, s")
                .to_string()
                .split(',')
                .map(str::trim)
                .flat_map(|key| key_map(key, InputAction::SoftDrop)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "hard_drop", "space")
                .to_string()
                .split(',')
                .map(str::trim)
                .flat_map(|key| key_map(key, InputAction::HardDrop)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "hold", "c")
                .to_string()
                .split(',')
                .map(str::trim)
                .flat_map(|key| key_map(key, InputAction::Hold)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "quit", "escape, q")
                .to_string()
                .split(',')
                .map(str::trim)
                .flat_map(|key| key_map(key, InputAction::Quit)));

            action_map
        };
    }
}

