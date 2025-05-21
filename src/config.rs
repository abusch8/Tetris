use std::net::SocketAddr;
use std::env;

use ini::Ini;
use home::home_dir;
use lazy_static::lazy_static;

lazy_static! {

    static ref CONFIG_PATH: String = env::var("TETRIS_CONFIG_PATH").unwrap_or(format!("{}/.config/tetris.ini", home_dir().unwrap().to_str().unwrap()));
    static ref CONFIG: Ini = Ini::load_from_file(&*CONFIG_PATH).unwrap_or(Ini::new());

    pub static ref MAX_FRAME_RATE: u64 = CONFIG
        .get_from_or(Some("display"), "max_frame_rate", "60")
        .parse()
        .unwrap_or_else(|_| panic!("Invalid max_frame_rate display config value"));

    pub static ref DISPLAY_FRAME_RATE: bool = CONFIG
        .get_from_or(Some("display"), "display_frame_rate", "false")
        .parse()
        .unwrap_or_else(|_| panic!("Invalid display_frame_rate display config value"));

    pub static ref USE_XTERM_256_COLORS: bool = CONFIG
        .get_from_or(Some("display"), "use_xterm_256_colors", "true")
        .parse()
        .unwrap_or_else(|_| panic!("Invalid use_xterm_256_colors display config value"));

    pub static ref ENABLE_MULTIPLAYER: bool = CONFIG
        .get_from_or(Some("experimental"), "enable_multiplayer", "false")
        .parse()
        .unwrap_or_else(|_| panic!("Invalid enable_multiplayer experiemental config value"));

    pub static ref BIND_ADDR: SocketAddr = CONFIG
        .get_from_or(Some("experimental"), "bind_addr", "127.0.0.1:8080")
        .parse::<SocketAddr>()
        .unwrap_or_else(|_| panic!("Invalid bind_addr experiemental config value"));

    pub static ref CONN_ADDR: SocketAddr = CONFIG
        .get_from_or(Some("experimental"), "conn_addr", "127.0.0.1:8081")
        .parse::<SocketAddr>()
        .unwrap_or_else(|_| panic!("Invalid conn_addr experiemental config value"));
}

pub mod controls {

    use std::collections::HashMap;
    use crossterm::event::KeyCode;
    use lazy_static::lazy_static;

    use crate::{config::CONFIG, event::Action};

    fn key_map(key: &str, action: Action) -> HashMap<KeyCode, Action> {
        let mut action_map = HashMap::new();
        let key = key.trim();
        if key.len() == 1 && key.is_ascii() {
            let char = key.chars().next().unwrap();
            action_map.insert(KeyCode::Char(char.to_ascii_uppercase()), action.clone());
            action_map.insert(KeyCode::Char(char.to_ascii_lowercase()), action);
        } else {
            match key {
                "up"        => action_map.insert(KeyCode::Up, action),
                "down"      => action_map.insert(KeyCode::Down, action),
                "left"      => action_map.insert(KeyCode::Left, action),
                "right"     => action_map.insert(KeyCode::Right, action),
                "space"     => action_map.insert(KeyCode::Char(' '), action),
                "escape"    => action_map.insert(KeyCode::Esc, action),
                _           => panic!("Invalid controls config key value: {}", key),
            };
        }
        action_map
    }

    lazy_static! {
        pub static ref ACTION_MAP: HashMap<KeyCode, Action> = {
            let mut action_map = HashMap::new();

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "move_right", "right")
                .to_string()
                .split(',')
                .flat_map(|key| key_map(key, Action::MoveRight)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "move_left", "left")
                .to_string()
                .split(',')
                .flat_map(|key| key_map(key, Action::MoveLeft)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "rotate_right", "up")
                .to_string()
                .split(',')
                .flat_map(|key| key_map(key, Action::RotateRight)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "rotate_left", "z")
                .to_string()
                .split(',')
                .flat_map(|key| key_map(key, Action::RotateLeft)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "soft_drop", "up")
                .to_string()
                .split(',')
                .flat_map(|key| key_map(key, Action::SoftDrop)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "hard_drop", "space")
                .to_string()
                .split(',')
                .flat_map(|key| key_map(key, Action::HardDrop)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "hold", "c")
                .to_string()
                .split(',')
                .flat_map(|key| key_map(key, Action::Hold)));

            action_map.extend(CONFIG
                .get_from_or(Some("controls"), "quit", "escape")
                .to_string()
                .split(',')
                .flat_map(|key| key_map(key, Action::Quit)));

            action_map
        };
    }
}

