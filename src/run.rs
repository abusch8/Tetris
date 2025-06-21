use std::{io::Result, time::{SystemTime, UNIX_EPOCH}};
use crossterm::event::EventStream;
use futures::{FutureExt, stream::StreamExt};
use tokio::{pin, select, time::{interval, sleep, Duration}};

use crate::{config, conn::{Conn, ConnType, TcpPacketMode, UdpPacketMode}, display::Display, event::handle_game_event, game::Game, tetromino::Geometry};

pub async fn run(conn_kind: ConnType, start_level: u32) -> Result<()> {
    let mut reader = EventStream::new();

    let display = &mut Display::new(conn_kind.is_multiplayer())?;

    pin! {
        let lock_delay = sleep(Duration::ZERO);
        let line_clear_delay_local = sleep(Duration::ZERO);
        let line_clear_delay_remote = sleep(Duration::ZERO);
    }

    let mut heartbeat_interval = interval(Duration::from_secs(1));
    let mut rtt = 0;

    let mut conn = Conn::establish_connection(conn_kind, display).await?;
    let game = &mut Game::start(conn_kind, start_level, &mut conn).await?;

    loop {
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
            _ = &mut lock_delay, if game.players.local.locking => {
                game.players.local.place(&mut line_clear_delay_local, &mut conn).await?;
            },
            _ = &mut line_clear_delay_local, if (
                game.players.local.clearing.len() > 0
            ) => {
                let clear_type = game.players.local.line_clear();
                if conn_kind.is_multiplayer() {
                    game.players.remote.as_mut().unwrap().add_garbage(clear_type);
                }
            },
            _ = &mut line_clear_delay_remote, if (
                conn_kind.is_multiplayer() &&
                game.players.remote.as_mut().unwrap().clearing.len() > 0
            ) => {
                let clear_type = game.players.remote.as_mut().unwrap().line_clear();
                game.players.local.add_garbage(clear_type);
            },
            _ = game.players.local.drop_interval.tick() => {
                game.players.local.drop(&mut lock_delay);
            },
            _ = async { game.players.remote.as_mut().unwrap().drop_interval.tick().await }, if (
                conn_kind.is_multiplayer()
            ) => {
                game.players.remote.as_mut().unwrap().drop(&mut lock_delay);
            },
            _ = display.render_interval.tick() => {
                display.render(game, rtt)?;
            },
            _ = display.frame_count_interval.tick(), if *config::DISPLAY_FRAME_RATE => {
                display.calc_fps();
            },
            _ = heartbeat_interval.tick(), if conn_kind.is_multiplayer() => {
                conn.send_ping().await?;
            },
            Ok((mode, payload)) = conn.recv_udp() => {
                match mode {
                    UdpPacketMode::Pos => {
                        let geometry_bytes: [u8; 41] = payload[0..41].try_into().unwrap();
                        let geometry = Geometry::from_bytes(geometry_bytes);
                        game.players.remote.as_mut().unwrap().set_falling_geometry(geometry);
                    },
                }
            },
            Ok((mode, payload)) = conn.recv_tcp() => {
                match mode {
                    TcpPacketMode::Ping => {
                        conn.send_pong(payload).await?;
                    },
                    TcpPacketMode::Pong => {
                        let ts_bytes: [u8; 16] = payload[0..16].try_into().unwrap();
                        let res_ts = u128::from_le_bytes(ts_bytes);
                        let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                        rtt = now_ts.saturating_sub(res_ts);
                    },
                    TcpPacketMode::Place => {
                        let geometry_bytes: [u8; 41] = payload[0..41].try_into().unwrap();
                        let geometry = Geometry::from_bytes(geometry_bytes);
                        game.players.remote.as_mut().unwrap().set_falling_geometry(geometry);
                        game.players.remote.as_mut().unwrap().place(&mut line_clear_delay_remote, &conn).await?;
                    },
                    TcpPacketMode::Hold => {
                        game.players.remote.as_mut().unwrap().hold(&conn).await?;
                    },
                    _ => (),
                }
            },
            _ = async {}, if (
                game.players.local.lost || conn_kind.is_multiplayer() &&
                game.players.remote.as_ref().unwrap().lost
            ) => {
                break;
            },
        }
    }
    Ok(())
}

