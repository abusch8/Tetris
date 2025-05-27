use std::{io::{Read, Result, Write}, net::{SocketAddr, TcpListener, TcpStream}};
use crossterm::event::EventStream;
use futures::{stream::StreamExt, FutureExt};
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};
use tokio::{net::{TcpSocket, UdpSocket}, pin, select, time::{interval, sleep, Duration, Interval}};

use crate::{config, debug_println, display::Display, event::handle_event, game::{Game, ShiftDirection}, tetromino::{CardinalDirection, Geometry}};

fn calc_drop_interval(level: u32) -> Interval {
    let drop_rate = (0.8 - (level - 1) as f32 * 0.007).powf((level - 1) as f32);
    let drop_duration = Duration::from_nanos((drop_rate * 1_000_000_000f32) as u64);

    interval(if drop_duration.is_zero() {
        Duration::from_nanos(1)
    } else {
        drop_duration
    })
}

fn tcp_listen(tcp_listener: &TcpListener) -> (TcpStream, SocketAddr) {
    match tcp_listener.accept() {
        Ok(socket) => socket,
        Err(_e) => tcp_listen(tcp_listener),
    }
}

pub async fn run(game: &mut Game, is_host: bool) -> Result<()> {
    let mut reader = EventStream::new();

    let display = &mut Display::new()?;
    display.draw()?;

    let frame_duration = Duration::from_nanos(if *config::MAX_FRAME_RATE > 0 {
        1_000_000_000 / *config::MAX_FRAME_RATE
    } else {
        1
    });

    let mut render_interval = interval(frame_duration);
    let mut drop_interval = calc_drop_interval(game.player[0].level);

    let mut prev_level = game.player[0].level;

    pin! {
        let lock_delay = sleep(Duration::ZERO);
        let line_clear_delay = sleep(Duration::ZERO);
    }

    let mut debug_frame_interval = interval(Duration::from_secs(1));
    let mut debug_frame = 0u64;

    let (mut tcp_stream, udp_socket) = if is_host {
        let tcp_listener = TcpListener::bind(*config::BIND_ADDR).unwrap();
        let (tcp_stream, peer_addr) = tcp_listen(&tcp_listener);

        let udp_socket = UdpSocket::bind(*config::BIND_ADDR).await?;
        udp_socket.connect(peer_addr).await?;

        (tcp_stream, udp_socket)
    } else {
        let tcp_stream = TcpStream::connect(*config::CONN_ADDR).unwrap();

        let udp_socket = UdpSocket::bind(tcp_stream.local_addr().unwrap()).await?;
        udp_socket.connect(*config::CONN_ADDR).await?;

        (tcp_stream, udp_socket)
    };

    let mut udp_buf = [0u8; 41];

    let seed = StdRng::seed_from_u64(if is_host {
        let seed: u32 = thread_rng().gen();
        tcp_stream.write_all(&seed.to_le_bytes())?;
        seed as u64
    } else {
        let mut seed_buf = [0u8; 4];
        tcp_stream.read_exact(&mut seed_buf)?;
        u32::from_le_bytes(seed_buf) as u64
    });

    Ok(loop {
        select! {
            Some(Ok(event)) = reader.next().fuse() => {
                handle_event(
                    game,
                    event,
                    display,
                    &mut lock_delay,
                    &mut line_clear_delay,
                    &udp_socket,
                ).await?
            },
            _ = &mut lock_delay, if game.player[0].locking => {
                game.player[0].place(&mut line_clear_delay);
            },
            _ = &mut line_clear_delay, if game.player[0].clearing.len() > 0 => {
                game.player[0].line_clear();
            },
            _ = drop_interval.tick() => {
                game.player[0].shift(ShiftDirection::Down, &mut lock_delay, &mut line_clear_delay);
            },
            _ = render_interval.tick() => {
                display.render(game)?;
                debug_frame += *config::DISPLAY_FRAME_RATE as u64;
            },
            _ = debug_frame_interval.tick(), if *config::DISPLAY_FRAME_RATE => {
                display.render_debug_info(debug_frame)?;
                debug_frame = 0;
            },
            _ = udp_socket.recv(&mut udp_buf) => {
                game.player[1].falling.geometry = Geometry::from_bytes(&udp_buf);
                debug_println!("{:?}", game.player[1].falling.geometry);

                // debug_println!("UDP shape:{:?} center:{:?}", shape, center);
            },
            _ = async {}, if game.player[0].level != prev_level => {
                prev_level = game.player[0].level;
                drop_interval = calc_drop_interval(game.player[0].level);
            },
            _ = async {}, if game.end => {
                break;
            },
        }
    })
}

