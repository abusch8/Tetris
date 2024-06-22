use lazy_static::lazy_static;
use ini::Ini;

lazy_static! {

    static ref CONFIG: Ini = Ini::load_from_file("conf.ini").unwrap();

    pub static ref MAX_FRAME_RATE: u64 = CONFIG
        .get_from_or(Some("display"), "max_frame_rate", "60")
        .parse::<u64>()
        .unwrap_or(60);

    pub static ref DISPLAY_FRAME_RATE: bool = CONFIG
        .get_from_or(Some("display"), "display_frame_rate", "false")
        .parse::<bool>()
        .unwrap_or(false);

    pub static ref USE_XTERM_256_COLORS: bool = CONFIG
        .get_from_or(Some("display"), "use_xterm_256_colors", "true")
        .parse::<bool>()
        .unwrap_or(true);

}

pub mod controls {

    use std::collections::HashSet;
    use lazy_static::lazy_static;
    use crossterm::event::KeyCode;

    use crate::config::CONFIG;

    fn key_map(key: &str) -> KeyCode {
        let key = key.trim();
        match key {
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "space" => KeyCode::Char(' '),
            _ => KeyCode::Char(key.chars().next().unwrap()),
        }
    }

    lazy_static! {

        pub static ref MOVE_RIGHT: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "move_right", "right")
            .to_string()
            .split(',')
            .map(key_map)
            .collect();

        pub static ref MOVE_LEFT: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "move_left", "left")
            .to_string()
            .split(',')
            .map(key_map)
            .collect();

        pub static ref ROTATE_RIGHT: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "rotate_right", "up")
            .to_string()
            .split(',')
            .map(key_map)
            .collect();

        pub static ref ROTATE_LEFT: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "rotate_left", "z")
            .to_string()
            .split(',')
            .map(key_map)
            .collect();

        pub static ref SOFT_DROP: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "soft_drop", "down")
            .to_string()
            .split(',')
            .map(key_map)
            .collect();

        pub static ref HARD_DROP: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "hard_drop", "space")
            .to_string()
            .split(',')
            .map(key_map)
            .collect();

        pub static ref HOLD: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "hold", "c")
            .to_string()
            .split(',')
            .map(key_map)
            .collect();
    }
}

