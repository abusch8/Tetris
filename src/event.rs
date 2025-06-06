use std::{io::Result, pin::Pin};
use crossterm::event::{Event, KeyEvent, KeyEventKind};
use tokio::time::Sleep;

use crate::{config, conn::{Conn, ConnTrait}, debug_println, display::Display, game::Game, player::{RotationDirection, ShiftDirection}};

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

pub async fn handle_event(
    game: &mut Game,
    conn: &mut Box<dyn ConnTrait>,
    event: Event,
    display: &mut Display,
    lock_delay: &mut Pin<&mut Sleep>,
    line_clear_delay: &mut Pin<&mut Sleep>,
) -> Result<()> {
    Ok(match event {
        Event::Key(KeyEvent { kind, code, .. }) => {
            if kind == KeyEventKind::Press {
                match config::controls::ACTION_MAP.get(&code) {
                    Some(Action::MoveRight) => {
                        game.player[0].shift(ShiftDirection::Right, lock_delay, line_clear_delay);
                        conn.send_pos(game).await?;
                    },
                    Some(Action::MoveLeft) => {
                        game.player[0].shift(ShiftDirection::Left, lock_delay, line_clear_delay);
                        conn.send_pos(game).await?;
                    },
                    Some(Action::RotateRight) => {
                        game.player[0].rotate(RotationDirection::Clockwise, lock_delay);
                        conn.send_pos(game).await?;
                    },
                    Some(Action::RotateLeft) => {
                        game.player[0].rotate(RotationDirection::CounterClockwise, lock_delay);
                        conn.send_pos(game).await?;
                    },
                    Some(Action::SoftDrop) => {
                        game.player[0].soft_drop(lock_delay, line_clear_delay);
                    },
                    Some(Action::HardDrop) => {
                        game.player[0].hard_drop(line_clear_delay);
                    },
                    Some(Action::Hold) => {
                        game.player[0].hold();
                    },
                    Some(Action::Quit) => {
                        game.player[0].lost = true;
                    },
                    None => (),
                }
            }
        },
        Event::Resize(_, _) => display.draw()?,
        _ => (),
    })
}

