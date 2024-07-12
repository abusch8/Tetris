use std::{io::Result, pin::Pin};
use crossterm::event::{Event, KeyEvent, KeyEventKind};
use tokio::time::Sleep;

use crate::{config, display::Display, game::{Game, RotationDirection, ShiftDirection}};

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

pub fn handle_event(event: Event, game: &mut Game, display: &mut Display, lock_delay: &mut Pin<&mut Sleep>, line_clear_delay: &mut Pin<&mut Sleep>) -> Result<()> {
    Ok(match event {
        Event::Key(KeyEvent { kind, code, .. }) => {
            if kind == KeyEventKind::Press {
                match config::controls::ACTION_MAP.get(&code) {
                    Some(Action::MoveRight) => {
                        game.shift(ShiftDirection::Right, lock_delay, line_clear_delay);
                    },
                    Some(Action::MoveLeft) => {
                        game.shift(ShiftDirection::Left, lock_delay, line_clear_delay);
                    },
                    Some(Action::RotateRight) => {
                        game.rotate(RotationDirection::Clockwise, lock_delay);
                    },
                    Some(Action::RotateLeft) => {
                        game.rotate(RotationDirection::CounterClockwise, lock_delay);
                    },
                    Some(Action::SoftDrop) => {
                        game.soft_drop(lock_delay, line_clear_delay);
                    },
                    Some(Action::HardDrop) => {
                        game.hard_drop(line_clear_delay);
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

