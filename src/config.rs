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

