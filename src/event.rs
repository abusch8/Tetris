use std::{io::Result, pin::Pin};
use crossterm::event::{Event, KeyEvent, KeyEventKind};
use tokio::time::{Instant, Sleep};

use crate::{config, display::Display, game::{Game, RotationDirection, ShiftDirection}, run::{LOCK_DURATION, LOCK_RESET_LIMIT}};

#[derive(Clone)]
pub enum Action {
    MoveRight,
    MoveLeft,
    RotateRight,
    RotateLeft,
    SoftDrop,
    HardDrop,
    Hold,
    Quit,
}

fn reset_lock_timer(game: &Game, lock_delay: &mut Pin<&mut Sleep>) {
    if game.lock_reset_count < LOCK_RESET_LIMIT {
        lock_delay.as_mut().reset(Instant::now() + LOCK_DURATION);
    }
}

pub fn handle_event(event: Event, game: &mut Game, display: &mut Display, lock_delay: &mut Pin<&mut Sleep>) -> Result<()> {
    Ok(match event {
        Event::Key(KeyEvent { kind, code, .. }) => {
            if kind == KeyEventKind::Press {
                match config::controls::ACTION_MAP.get(&code) {
                    Some(Action::MoveRight) => {
                        game.shift(ShiftDirection::Right);
                        if !game.hitting_right(&game.falling) {
                            reset_lock_timer(&game, lock_delay);
                        }
                    },
                    Some(Action::MoveLeft) => {
                        game.shift(ShiftDirection::Left);
                        if !game.hitting_left(&game.falling) {
                            reset_lock_timer(&game, lock_delay);
                        }
                    },
                    Some(Action::RotateRight) => {
                        game.rotate(RotationDirection::Clockwise);
                        reset_lock_timer(&game, lock_delay);
                    },
                    Some(Action::RotateLeft) => {
                        game.rotate(RotationDirection::CounterClockwise);
                        reset_lock_timer(&game, lock_delay);
                    },
                    Some(Action::SoftDrop) => {
                        if !game.hitting_bottom(&game.falling) {
                            lock_delay.as_mut().reset(Instant::now() + LOCK_DURATION);
                        }
                        game.soft_drop();
                    },
                    Some(Action::HardDrop) => {
                        game.hard_drop();
                    },
                    Some(Action::Hold) => {
                        game.hold();
                    },
                    Some(Action::Quit) => {
                        game.end = true;
                    },
                    None => (),
                }
            }
        },
        Event::Resize(_, _) => display.draw()?,
        _ => (),
    })
}

