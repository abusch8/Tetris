use std::{io::Result, time::{SystemTime, UNIX_EPOCH}};
use crossterm::event::EventStream;
use futures::{FutureExt, stream::StreamExt};
use tokio::{select, time::{interval, sleep, Duration}};

use crate::{agent::Agent, config, conn::{Conn, ConnKind, TcpPacketMode, UdpPacketMode}, display::Display, event::handle_game_event, game::Game, tetromino::Geometry, Mode};

pub async fn run(mode: Mode, conn_kind: ConnKind, start_level: u32) -> Result<()> {
    let mut reader = EventStream::new();

    let display = &mut Display::new(mode)?;

    let mut heartbeat_interval = interval(Duration::from_secs(1));
    let mut rtt = 0;

    let mut conn = Conn::establish_connection(conn_kind, display).await?;
    let game = &mut Game::start(mode, start_level, conn_kind, &mut conn).await?;

    let player = &mut game.player;

    macro_rules! game_select {
        ($($branch:tt)*) => {
            select! {
                Some(Ok(event)) = reader.next().fuse() => {
                    handle_game_event(player, &conn, event, display).await?
                },
                _ = display.frame_count_interval.tick(), if *config::DISPLAY_FRAME_RATE => {
                    display.calc_fps();
                },
                _ = player.drop_interval.tick() => {
                    player.drop();
                },
                _ = &mut player.timers.lock_delay, if player.locking => {
                    player.place(&mut conn).await?;
                },
                $($branch)*
            }
        };
    }

    macro_rules! opponent_game_select {
        ($opponent:ident, $($branch:tt)*) => {
            game_select! {
                _ = display.render_interval.tick() => {
                    display.render(&vec![player, $opponent], rtt)?;
                },
                _ = &mut player.timers.line_clear_delay, if player.clearing.len() > 0 => {
                    let clear_kind = player.line_clear();
                    $opponent.add_garbage(clear_kind);
                },
                _ = $opponent.drop_interval.tick() => {
                    $opponent.drop();
                },
                _ = &mut $opponent.timers.line_clear_delay, if $opponent.clearing.len() > 0 => {
                    let clear_kind = $opponent.line_clear();
                    player.add_garbage(clear_kind);
                },
                _ = async {}, if player.lost || $opponent.lost => {
                    break;
                },
                $($branch)*
            }
        }
    }

    match mode {
        Mode::Singleplayer => {
            loop {
                game_select! {
                    _ = display.render_interval.tick() => {
                        display.render(&vec![player], rtt)?;
                    },
                    _ = &mut player.timers.line_clear_delay, if player.clearing.len() > 0 => {
                        let _ = player.line_clear();
                    },
                    _ = async {}, if player.lost => {
                        break;
                    },
                }
            }
        },
        Mode::Multiplayer => {
            if let Some(opponent) = game.opponent.as_mut() {
                loop {
                    opponent_game_select!(opponent,
                        _ = heartbeat_interval.tick() => {
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
                    )
                }
            }
        },
        Mode::PlayerVsComputer => {
            let mut agent = Agent::new();
            let mut agent_delay = Box::pin(sleep(Duration::ZERO));
            if let Some(opponent) = game.opponent.as_mut() {
                agent.evaluate(opponent);
                loop {
                    opponent_game_select!(opponent,
                        _ = &mut agent_delay => {
                            agent.execute(opponent, &conn).await?;
                            agent_delay.set(sleep(Duration::from_millis(200)));
                        },
                    )
                }
            }
        },
        Mode::ComputerVsComputer => {
            let mut p1_agent = Agent::new();
            let mut p2_agent = Agent::new();
            let mut agent_delay = Box::pin(sleep(Duration::ZERO));
            if let Some(opponent) = game.opponent.as_mut() {
                p1_agent.evaluate(player);
                p2_agent.evaluate(opponent);
                loop {
                    opponent_game_select!(opponent,
                        _ = &mut agent_delay => {
                            p1_agent.execute(player, &conn).await?;
                            p2_agent.execute(opponent, &conn).await?;
                            agent_delay.set(sleep(Duration::from_millis(200)));
                        },
                    )
                }
            }
        },
    }
    Ok(())
}

