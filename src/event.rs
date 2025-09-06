use std::{io::Result, process::exit};
use crossterm::event::{Event, KeyEvent, KeyEventKind};

use crate::{config, conn::ConnTrait, display::Display, exit_tui_mode, player::{Player, ShiftDirection}, tetromino::RotationDirection};

#[derive(Clone)]
pub enum InputAction {
    MoveRight,
    MoveLeft,
    RotateRight,
    RotateLeft,
    SoftDrop,
    HardDrop,
    Hold,
    Quit,
}

pub async fn handle_game_event(
    player: &mut Player,
    conn: &Box<dyn ConnTrait>,
    event: Event,
    display: &mut Display,
) -> Result<()> {
    Ok(match event {
        Event::Key(KeyEvent { kind, code, .. }) => {
            if kind == KeyEventKind::Press {
                match config::controls::ACTION_MAP.get(&code) {
                    Some(InputAction::MoveRight) => {
                        player.shift(ShiftDirection::Right, conn).await?;
                    },
                    Some(InputAction::MoveLeft) => {
                        player.shift(ShiftDirection::Left, conn).await?;
                    },
                    Some(InputAction::RotateRight) => {
                        player.rotate(RotationDirection::Clockwise, conn).await?;
                    },
                    Some(InputAction::RotateLeft) => {
                        player.rotate(RotationDirection::CounterClockwise, conn).await?;
                    },
                    Some(InputAction::SoftDrop) => {
                        player.soft_drop(conn).await?;
                    },
                    Some(InputAction::HardDrop) => {
                        player.hard_drop(conn).await?;
                    },
                    Some(InputAction::Hold) => {
                        player.hold(conn).await?;
                    },
                    Some(InputAction::Quit) => {
                        player.lost = true;
                    },
                    None => (),
                }
            }
        },
        Event::Resize(_, _) => display.resize()?,
        _ => (),
    })
}

pub fn handle_conn_event(event: Event, display: &mut Display) -> Result<()> {
    Ok(match event {
        Event::Key(KeyEvent { kind, code, .. }) => {
            if kind == KeyEventKind::Press {
                match config::controls::ACTION_MAP.get(&code) {
                    Some(InputAction::Quit) => {
                        exit_tui_mode()?;
                        exit(0);
                    },
                    Some(_) => (),
                    None => (),
                }
            }
        }
        Event::Resize(_, _) => display.resize()?,
        _ => (),
    })
}

