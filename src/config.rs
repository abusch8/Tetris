use ini::Ini;
use home::home_dir;
use lazy_static::lazy_static;

lazy_static! {

    static ref CONFIG_PATH: String = format!("{}/.config/tetris.ini", home_dir().unwrap().to_str().unwrap());

    static ref CONFIG: Ini = Ini::load_from_file(&*CONFIG_PATH).unwrap_or(Ini::new());

    pub static ref MAX_FRAME_RATE: u64 = CONFIG
        .get_from_or(Some("display"), "max_frame_rate", "60")
        .parse()
        .unwrap_or(60);

    pub static ref DISPLAY_FRAME_RATE: bool = CONFIG
        .get_from_or(Some("display"), "display_frame_rate", "false")
        .parse()
        .unwrap_or(false);

    pub static ref USE_XTERM_256_COLORS: bool = CONFIG
        .get_from_or(Some("display"), "use_xterm_256_colors", "true")
        .parse()
        .unwrap_or(true);

}

pub mod controls {

    use std::collections::HashSet;
    use crossterm::event::KeyCode;
    use lazy_static::lazy_static;

    use crate::config::CONFIG;

    fn key_map(key: &str) -> HashSet<KeyCode> {
        let key = key.trim();
        match key {
            "up" => HashSet::from([KeyCode::Up]),
            "down" => HashSet::from([KeyCode::Down]),
            "left" => HashSet::from([KeyCode::Left]),
            "right" => HashSet::from([KeyCode::Right]),
            "space" => HashSet::from([KeyCode::Char(' ')]),
            "escape" => HashSet::from([KeyCode::Esc]),
            _ => {
                if key.len() > 1 {
                    panic!("ERROR: Invalid controls config value `{}`", key);
                }
                let char = key.chars().next().unwrap();
                if char.is_ascii_alphabetic() {
                    HashSet::from([
                        KeyCode::Char(char.to_lowercase().next().unwrap()),
                        KeyCode::Char(char.to_uppercase().next().unwrap()),
                    ])
                } else {
                    HashSet::from([KeyCode::Char(char)])
                }
            },
        }
    }

    lazy_static! {

        pub static ref MOVE_RIGHT: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "move_right", "right")
            .to_string()
            .split(',')
            .flat_map(key_map)
            .collect();

        pub static ref MOVE_LEFT: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "move_left", "left")
            .to_string()
            .split(',')
            .flat_map(key_map)
            .collect();

        pub static ref ROTATE_RIGHT: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "rotate_right", "up")
            .to_string()
            .split(',')
            .flat_map(key_map)
            .collect();

        pub static ref ROTATE_LEFT: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "rotate_left", "z")
            .to_string()
            .split(',')
            .flat_map(key_map)
            .collect();

        pub static ref SOFT_DROP: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "soft_drop", "down")
            .to_string()
            .split(',')
            .flat_map(key_map)
            .collect();

        pub static ref HARD_DROP: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "hard_drop", "space")
            .to_string()
            .split(',')
            .flat_map(key_map)
            .collect();

        pub static ref HOLD: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "hold", "c")
            .to_string()
            .split(',')
            .flat_map(key_map)
            .collect();

        pub static ref QUIT: HashSet<KeyCode> = CONFIG
            .get_from_or(Some("controls"), "quit", "escape, q")
            .to_string()
            .split(',')
            .flat_map(key_map)
            .collect();
    }
}

