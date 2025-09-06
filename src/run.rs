use std::{io::Result, time::{SystemTime, UNIX_EPOCH}};
use crossterm::event::EventStream;
use futures::{FutureExt, stream::StreamExt};
use tokio::{pin, select, time::{interval, sleep, Duration}};

use crate::{agent::Agent, config, conn::{Conn, ConnKind, TcpPacketMode, UdpPacketMode}, display::Display, event::handle_game_event, game::Game, player::PlayerKind, tetromino::Geometry};
use crate::debug_log;

pub async fn run(ai: bool, conn_kind: ConnKind, start_level: u32) -> Result<()> {
    let mut reader = EventStream::new();

    let display = &mut Display::new(ai || conn_kind.is_multiplayer())?;

    pin! {
        let agent_delay = sleep(Duration::ZERO);
    }

    let mut heartbeat_interval = interval(Duration::from_secs(1));
    let mut rtt = 0;

    let mut conn = Conn::establish_connection(conn_kind, display).await?;
    let game = &mut Game::start(ai, conn_kind, start_level, &mut conn).await?;

    let mut agent = if ai {
        let mut agent = Agent::new();
        agent.evaluate(game.opponent.as_ref().unwrap());
        Some(agent)
    } else {
        None
    };

    let player = &mut game.player;
    let opponent = game.opponent.as_mut().unwrap();

    loop {
        select! {
            Some(Ok(event)) = reader.next().fuse() => {
                handle_game_event(player, &conn, event, display).await?
            },
            _ = &mut player.timers.lock_delay, if player.locking => {
                player.place(&mut conn).await?;
            },
            _ = &mut player.timers.line_clear_delay, if (
                player.clearing.len() > 0
            ) => {
                let clear_kind = player.line_clear();
                if conn_kind.is_multiplayer() {
                    opponent.add_garbage(clear_kind);
                }
            },
            _ = &mut opponent.timers.line_clear_delay, if (
                (ai || conn_kind.is_multiplayer()) &&
                opponent.clearing.len() > 0
            ) => {
                let clear_kind = opponent.line_clear();
                player.add_garbage(clear_kind);
            },
            _ = &mut agent_delay, if ai => {
                agent.as_mut().unwrap().execute(opponent, &conn).await?;
                agent_delay.set(sleep(Duration::from_millis(200)));
            },
            _ = player.drop_interval.tick() => {
                player.drop();
            },
            _ = opponent.drop_interval.tick(), if (
                ai || conn_kind.is_multiplayer()
            ) => {
                opponent.drop();
            },
            _ = display.render_interval.tick() => {
                // debug_log!("{:?}", game.player.falling.geometry.center);
                display.render(vec![player, opponent], rtt)?;
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
                        opponent.set_falling_geometry(geometry);
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
                        opponent.set_falling_geometry(geometry);
                        opponent.place(&conn).await?;
                    },
                    TcpPacketMode::Hold => {
                        opponent.hold(&conn).await?;
                    },
                    _ => (),
                }
            },
            _ = async {}, if (
                player.lost || (ai || conn_kind.is_multiplayer()) &&
                opponent.lost
            ) => {
                break;
            },
        }
    }
    Ok(())
}

