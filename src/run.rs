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
        let lock_delay = sleep(Duration::ZERO);
        let line_clear_delay = sleep(Duration::ZERO);
        let line_clear_delay_opponent = sleep(Duration::ZERO);
        let agent_delay = sleep(Duration::ZERO);
    }

    let mut heartbeat_interval = interval(Duration::from_secs(1));
    let mut rtt = 0;

    let mut conn = Conn::establish_connection(conn_kind, display).await?;
    let game = &mut Game::start(ai, conn_kind, start_level, &mut conn).await?;

    let mut agent = if ai {
        let mut agent = Agent::new();
        agent.evaluate(game.players.opponent.as_ref().unwrap());
        Some(agent)
    } else {
        None
    };

    loop {
        select! {
            Some(Ok(event)) = reader.next().fuse() => {
                handle_game_event(
                    game,
                    &conn,
                    event,
                    display,
                    &mut lock_delay,
                    &mut line_clear_delay,
                ).await?
            },
            _ = &mut lock_delay, if game.players.main.locking => {
                game.players.main.place(&mut line_clear_delay, &mut conn).await?;
            },
            _ = &mut line_clear_delay, if (
                game.players.main.clearing.len() > 0
            ) => {
                let clear_kind = game.players.main.line_clear();
                if conn_kind.is_multiplayer() {
                    game.players.opponent.as_mut().unwrap().add_garbage(clear_kind);
                }
            },
            _ = &mut line_clear_delay_opponent, if (
                (ai || conn_kind.is_multiplayer()) &&
                game.players.opponent.as_mut().unwrap().clearing.len() > 0
            ) => {
                let clear_kind = game.players.opponent.as_mut().unwrap().line_clear();
                game.players.main.add_garbage(clear_kind);
            },
            _ = &mut agent_delay, if ai => {
                agent.as_mut().unwrap().execute(game.players.opponent.as_mut().unwrap(), &mut lock_delay, &mut line_clear_delay_opponent, &conn).await?;
                agent_delay.set(sleep(Duration::from_millis(200)));
            },
            _ = game.players.main.drop_interval.tick() => {
                game.players.main.drop(&mut lock_delay);
            },
            _ = async { game.players.opponent.as_mut().unwrap().drop_interval.tick().await }, if (
                ai || conn_kind.is_multiplayer()
            ) => {
                game.players.opponent.as_mut().unwrap().drop(&mut lock_delay);
            },
            _ = display.render_interval.tick() => {
                // debug_log!("{:?}", game.players.main.falling.geometry.center);
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
                        game.players.opponent.as_mut().unwrap().set_falling_geometry(geometry);
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
                        game.players.opponent.as_mut().unwrap().set_falling_geometry(geometry);
                        game.players.opponent.as_mut().unwrap().place(&mut line_clear_delay_opponent, &conn).await?;
                    },
                    TcpPacketMode::Hold => {
                        game.players.opponent.as_mut().unwrap().hold(&conn).await?;
                    },
                    _ => (),
                }
            },
            _ = async {}, if (
                game.players.main.lost || (ai || conn_kind.is_multiplayer()) &&
                game.players.opponent.as_ref().unwrap().lost
            ) => {
                break;
            },
        }
    }
    Ok(())
}

