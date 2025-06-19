use std::time::SystemTime;
use crossterm::style::Color;

use crate::{config, game::Game, player::Player};

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match (h / 60.0).floor() as u8 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 => (c, 0.0, x),
        _ => (0.0, 0.0, 0.0),
    };

    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

fn radio_spectrum_gradient(index: u16, steps: u16) -> Color {
    let half = steps as f32 / 2.0;
    let i = index as f32;

    let hue = if i <= half {
        (i / half) * 360.0
    } else {
        ((steps as f32 - 1.0 - i) / half) * 360.0
    } % 360.0;

    let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);

    Color::Rgb { r, g ,b }
}

fn linear_gradient(index: u16, steps: u16, start: (u8, u8, u8), end: (u8, u8, u8)) -> Color  {
    let half = steps as f32 / 2.0;
    let i = index as f32;
    let t = if i <= half {
        i / half
    } else {
        (steps as f32 - 1.0 - i) / half
    };

    let r = (start.0 as f32 + t * (end.0 as f32 - start.0 as f32)).round() as u8;
    let g = (start.1 as f32 + t * (end.1 as f32 - start.1 as f32)).round() as u8;
    let b = (start.2 as f32 + t * (end.2 as f32 - start.2 as f32)).round() as u8;

    Color::Rgb { r, g, b }
}

pub fn get_board_color(player: &Player, y: u16) -> Color {
    fn time_index() -> u128 {
        SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() / 30
    }
    if *config::PARTY_MODE {
        radio_spectrum_gradient(((y as u128 + time_index()) % 160) as u16, 160)
    } else if player.score.combo >= 0 {
        linear_gradient(((y as u128 + time_index()) % 40) as u16, 40, (255, 255, 255), (0, 183, 235))
    } else {
        Color::White
    }
}

