use std::{io::Result, pin::Pin};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use tokio::time::{Instant, Sleep};

use crate::{display::Display, Game, RotationDirection, ShiftDirection, LOCK_DURATION, LOCK_RESET_LIMIT};

fn reset_lock_timer(game: &Game, lock_delay: &mut Pin<&mut Sleep>) {
    if game.lock_reset_count < LOCK_RESET_LIMIT {
        lock_delay.as_mut().reset(Instant::now() + LOCK_DURATION);
    }
}

fn key_map(key: &str) -> KeyCode {
    match key {
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "space" => KeyCode::Char(' '),
        _ => KeyCode::Char(key.chars().next().unwrap()),
    }
}

pub fn handle_event(event: Event, game: &mut Game, display: &mut Display, lock_delay: &mut Pin<&mut Sleep>) -> Result<()> {
    Ok(match event {
        Event::Key(KeyEvent { kind, code, .. }) => {
            if kind == KeyEventKind::Press {
                match code {
                    KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                        game.rotate(RotationDirection::Clockwise);
                        reset_lock_timer(&game, lock_delay);
                    },
                    KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                        game.shift(ShiftDirection::Left);
                        if !game.hitting_left(&game.falling) {
                            reset_lock_timer(&game, lock_delay);
                        }
                    },
                    KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                        if !game.hitting_bottom(&game.falling) {
                            lock_delay.as_mut().reset(Instant::now() + LOCK_DURATION);
                        }
                        game.soft_drop();
                    },
                    KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                        game.shift(ShiftDirection::Right);
                        if !game.hitting_right(&game.falling) {
                            reset_lock_timer(&game, lock_delay);
                        }
                    },
                    KeyCode::Char(' ') => {
                        game.hard_drop();
                    },
                    KeyCode::Char('z') | KeyCode::Char('Z') => {
                        game.rotate(RotationDirection::CounterClockwise);
                    },
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        game.hold();
                    },
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        game.end = true;
                    },
                    _ => (),
                }
            }
        },
        Event::Resize(_, _) => display.draw()?,
        _ => (),
    })
}
