use std::{io::Result, pin::Pin};
use crossterm::event::{Event, KeyEvent, KeyEventKind};
use tokio::{net::UdpSocket, time::Sleep};

use crate::{config, debug_println, display::Display, game::{Game, RotationDirection, ShiftDirection}};

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

async fn send_pos(udp_socket: &UdpSocket, game: &Game) -> Result<()> {
    udp_socket.send(&game.player[0].falling.geometry.to_bytes()).await?;
    Ok(())
}

pub async fn handle_event(
    game: &mut Game,
    event: Event,
    display: &mut Display,
    lock_delay: &mut Pin<&mut Sleep>,
    line_clear_delay: &mut Pin<&mut Sleep>,
    udp_socket: &UdpSocket,
) -> Result<()> {
    Ok(match event {
        Event::Key(KeyEvent { kind, code, .. }) => {
            if kind == KeyEventKind::Press {
                match config::controls::ACTION_MAP.get(&code) {
                    Some(Action::MoveRight) => {
                        game.player[0].shift(ShiftDirection::Right, lock_delay, line_clear_delay);
                        send_pos(udp_socket, game).await?;
                    },
                    Some(Action::MoveLeft) => {
                        game.player[0].shift(ShiftDirection::Left, lock_delay, line_clear_delay);
                        send_pos(udp_socket, game).await?;
                    },
                    Some(Action::RotateRight) => {
                        game.player[0].rotate(RotationDirection::Clockwise, lock_delay);
                        send_pos(udp_socket, game).await?;
                    },
                    Some(Action::RotateLeft) => {
                        game.player[0].rotate(RotationDirection::CounterClockwise, lock_delay);
                        send_pos(udp_socket, game).await?;
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

