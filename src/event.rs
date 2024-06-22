use std::{io::Result, pin::Pin};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use tokio::time::{Instant, Sleep};

use crate::{config, display::Display, Game, RotationDirection, ShiftDirection, LOCK_DURATION, LOCK_RESET_LIMIT};

fn reset_lock_timer(game: &Game, lock_delay: &mut Pin<&mut Sleep>) {
    if game.lock_reset_count < LOCK_RESET_LIMIT {
        lock_delay.as_mut().reset(Instant::now() + LOCK_DURATION);
    }
}

pub fn handle_event(event: Event, game: &mut Game, display: &mut Display, lock_delay: &mut Pin<&mut Sleep>) -> Result<()> {
    Ok(match event {
        Event::Key(KeyEvent { kind, code, .. }) => {
            if kind == KeyEventKind::Press {
                match code {
                    code if config::controls::ROTATE_RIGHT.contains(&code) => {
                        game.rotate(RotationDirection::Clockwise);
                        reset_lock_timer(&game, lock_delay);
                    },
                    code if config::controls::MOVE_LEFT.contains(&code) => {
                        game.shift(ShiftDirection::Left);
                        if !game.hitting_left(&game.falling) {
                            reset_lock_timer(&game, lock_delay);
                        }
                    },
                    code if config::controls::SOFT_DROP.contains(&code) => {
                        if !game.hitting_bottom(&game.falling) {
                            lock_delay.as_mut().reset(Instant::now() + LOCK_DURATION);
                        }
                        game.soft_drop();
                    },
                    code if config::controls::MOVE_RIGHT.contains(&code) => {
                        game.shift(ShiftDirection::Right);
                        if !game.hitting_right(&game.falling) {
                            reset_lock_timer(&game, lock_delay);
                        }
                    },
                    code if config::controls::HARD_DROP.contains(&code) => {
                        game.hard_drop();
                    },
                    code if config::controls::ROTATE_LEFT.contains(&code) => {
                        game.rotate(RotationDirection::CounterClockwise);
                    },
                    code if config::controls::HOLD.contains(&code) => {
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
