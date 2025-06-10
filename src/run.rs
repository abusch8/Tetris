use std::{io::Result, time::{SystemTime, UNIX_EPOCH}};
use crossterm::event::EventStream;
use futures::{FutureExt, stream::StreamExt};
use tokio::{pin, select, time::{interval, sleep, Duration, Interval}};

use crate::{config, conn::{Conn, ConnKind, TcpPacketMode, UdpPacketMode}, display::Display, event::handle_game_event, game::Game, player::PlayerKind, tetromino::Geometry};

fn calc_drop_interval(level: u32) -> Interval {
    let drop_rate = (0.8 - (level - 1) as f32 * 0.007).powf((level - 1) as f32);
    let drop_duration = Duration::from_nanos((drop_rate * 1_000_000_000f32) as u64);

    interval(if drop_duration.is_zero() {
        Duration::from_nanos(1)
    } else {
        drop_duration
    })
}

pub async fn run(conn_kind: ConnKind, start_level: u32) -> Result<()> {
    let mut reader = EventStream::new();

    let display = &mut Display::new(conn_kind.is_multiplayer())?;
    display.draw()?;

    let frame_duration = Duration::from_nanos(if *config::MAX_FRAME_RATE > 0 {
        1_000_000_000 / *config::MAX_FRAME_RATE
    } else {
        1
    });

    pin! {
        let lock_delay = sleep(Duration::ZERO);
        let line_clear_delay_local = sleep(Duration::ZERO);
        let line_clear_delay_remote = sleep(Duration::ZERO);
    }


    let mut debug_frame_interval = interval(Duration::from_secs(1));
    let mut debug_frame = 0u64;

    let mut heartbeat_interval = interval(Duration::from_secs(1));
    let mut rtt = 0;

    let mut conn = Conn::establish_connection(conn_kind, display).await?;

    let game = &mut Game::start(conn_kind, start_level, &mut conn).await?;

    let mut render_interval = interval(frame_duration);
    let mut drop_interval = calc_drop_interval(game.players[PlayerKind::Local].level);

    let mut prev_level = game.players[PlayerKind::Local].level;

    Ok(loop {
        select! {
            Some(Ok(event)) = reader.next().fuse() => {
                handle_game_event(
                    game,
                    &conn,
                    event,
                    display,
                    &mut lock_delay,
                    &mut line_clear_delay_local,
                ).await?
            },
            _ = &mut lock_delay, if game.players[PlayerKind::Local].locking => {
                game.players[PlayerKind::Local].place(&mut line_clear_delay_local, &mut conn).await?;
            },
            _ = &mut line_clear_delay_local, if (
                game.players[PlayerKind::Local].clearing.len() > 0
            ) => {
                game.players[PlayerKind::Local].line_clear();
            },
            _ = &mut line_clear_delay_remote, if (
                conn_kind.is_multiplayer() &&
                game.players[PlayerKind::Remote].clearing.len() > 0
            ) => {
                game.players[PlayerKind::Remote].line_clear();
            },
            _ = drop_interval.tick() => {
                for p in game.players.iter_mut() {
                    p.drop(&mut lock_delay);
                }
            },
            _ = render_interval.tick() => {
                display.render(game)?;
                debug_frame += *config::DISPLAY_FRAME_RATE as u64;
            },
            _ = debug_frame_interval.tick(), if *config::DISPLAY_FRAME_RATE => {
                display.render_debug_info(debug_frame, rtt)?;
                debug_frame = 0;
            },
            _ = heartbeat_interval.tick(), if conn_kind.is_multiplayer() => {
                conn.send_ping().await?;
            },
            Ok((mode, payload)) = conn.recv_udp() => {
                match mode {
                    UdpPacketMode::Pos => {
                        let geometry_bytes: &[u8; 41] = payload[0..41].try_into().unwrap();
                        let geometry = Geometry::from_bytes(geometry_bytes);
                        game.players[PlayerKind::Remote].set_falling_geometry(geometry);
                    },
                }
            },
            Ok((mode, payload)) = conn.recv_tcp() => {
                match mode {
                    TcpPacketMode::Ping => {
                        conn.send_pong(&payload).await?;
                    },
                    TcpPacketMode::Pong => {
                        let ts_bytes: [u8; 16] = payload[0..16].try_into().unwrap();
                        let res_ts = u128::from_le_bytes(ts_bytes);
                        let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                        rtt = now_ts - res_ts;
                    },
                    TcpPacketMode::Place => {
                        let geometry_bytes: &[u8; 41] = payload[0..41].try_into().unwrap();
                        let geometry = Geometry::from_bytes(geometry_bytes);
                        game.players[PlayerKind::Remote].set_falling_geometry(geometry);
                        game.players[PlayerKind::Remote].place(&mut line_clear_delay_remote, &conn).await?;
                    },
                    TcpPacketMode::Hold => {
                        game.players[PlayerKind::Remote].hold(&conn).await?;
                    },
                    _ => (),
                }
            },
            _ = async {}, if game.players[PlayerKind::Local].level != prev_level => {
                prev_level = game.players[PlayerKind::Local].level;
                drop_interval = calc_drop_interval(game.players[PlayerKind::Local].level);
            },
            _ = async {}, if game.players.iter().any(|p| p.lost) => {
                break;
            },
        }
    })
}

