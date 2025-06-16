use std::{io::Result, pin::Pin, process::exit};
use crossterm::event::{Event, KeyEvent, KeyEventKind};
use tokio::time::Sleep;

use crate::{config, conn::ConnTrait, display::Display, exit_tui_mode, game::Game, player::ShiftDirection, tetromino::RotationDirection};

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

pub async fn handle_game_event(
    game: &mut Game,
    conn: &Box<dyn ConnTrait>,
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
                        game.players.local.shift(
                            ShiftDirection::Right,
                            lock_delay,
                            line_clear_delay,
                            conn,
                        ).await?;
                    },
                    Some(Action::MoveLeft) => {
                        game.players.local.shift(
                            ShiftDirection::Left,
                            lock_delay,
                            line_clear_delay,
                            conn,
                        ).await?;
                    },
                    Some(Action::RotateRight) => {
                        game.players.local.rotate(
                            RotationDirection::Clockwise,
                            lock_delay,
                            conn,
                        ).await?;
                    },
                    Some(Action::RotateLeft) => {
                        game.players.local.rotate(
                            RotationDirection::CounterClockwise,
                            lock_delay,
                            conn,
                        ).await?;
                    },
                    Some(Action::SoftDrop) => {
                        game.players.local.soft_drop(
                            lock_delay,
                            conn,
                        ).await?;
                    },
                    Some(Action::HardDrop) => {
                        game.players.local.hard_drop(
                            line_clear_delay,
                            conn,
                        ).await?;
                    },
                    Some(Action::Hold) => {
                        game.players.local.hold(
                            conn,
                        ).await?;
                    },
                    Some(Action::Quit) => {
                        game.players.local.lost = true;
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
                    Some(Action::Quit) => {
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

