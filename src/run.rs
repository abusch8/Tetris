use std::{io::Result, pin::Pin};
use crossterm::event::EventStream;
use futures::{future, FutureExt, stream::StreamExt};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, pin, select, time::{interval, sleep, Duration, Interval}};

use crate::{config, conn::{Conn, ConnTrait, DummyConn}, debug_println, display::Display, event::handle_event, game::Game, player::ShiftDirection, tetromino::Geometry};

fn calc_drop_interval(level: u32) -> Interval {
    let drop_rate = (0.8 - (level - 1) as f32 * 0.007).powf((level - 1) as f32);
    let drop_duration = Duration::from_nanos((drop_rate * 1_000_000_000f32) as u64);

    interval(if drop_duration.is_zero() {
        Duration::from_nanos(1)
    } else {
        drop_duration
    })
}

pub async fn run(level: u32, is_host: bool) -> Result<()> {
    let mut reader = EventStream::new();

    let display = &mut Display::new()?;
    display.draw()?;

    let frame_duration = Duration::from_nanos(if *config::MAX_FRAME_RATE > 0 {
        1_000_000_000 / *config::MAX_FRAME_RATE
    } else {
        1
    });

    pin! {
        let lock_delay = sleep(Duration::ZERO);
        let line_clear_delay = sleep(Duration::ZERO);
    }

    let mut debug_frame_interval = interval(Duration::from_secs(1));
    let mut debug_frame = 0u64;

    let mut conn: Box<dyn ConnTrait> = if *config::ENABLE_MULTIPLAYER {
        Box::new(Conn::establish_connection(is_host).await?)
    } else {
        Box::new(DummyConn)
    };

    let game = &mut Game::start(level, &mut conn).await?;

    let mut render_interval = interval(frame_duration);
    let mut drop_interval = calc_drop_interval(game.player[0].level);

    let mut prev_level = game.player[0].level;

    Ok(loop {
        select! {
            Some(Ok(event)) = reader.next().fuse() => {
                handle_event(
                    game,
                    &mut conn,
                    event,
                    display,
                    &mut lock_delay,
                    &mut line_clear_delay,
                ).await?
            },
            _ = &mut lock_delay, if game.player[0].locking => {
                game.player[0].place(&mut line_clear_delay);
                conn.send_place().await?;
            },
            _ = &mut line_clear_delay, if game.player[0].clearing.len() > 0 => {
                game.player[0].line_clear();
            },
            _ = drop_interval.tick() => {
                game.player
                    .iter_mut()
                    .for_each(|p| p.shift(ShiftDirection::Down, &mut lock_delay, &mut line_clear_delay));
            },
            _ = render_interval.tick() => {
                display.render(game)?;
                debug_frame += *config::DISPLAY_FRAME_RATE as u64;
            },
            _ = debug_frame_interval.tick(), if *config::DISPLAY_FRAME_RATE => {
                display.render_debug_info(debug_frame)?;
                debug_frame = 0;
            },
            Ok(geometry) = conn.recv_pos() => {
                game.player[1].falling.geometry = geometry;
                game.player[1].update_ghost();
            },
            Ok(_) = conn.recv_place() => {
                game.player[1].hard_drop(&mut line_clear_delay);
            },
            _ = async {}, if game.player[0].level != prev_level => {
                prev_level = game.player[0].level;
                drop_interval = calc_drop_interval(game.player[0].level);
            },
            _ = async {}, if game.player.iter().any(|p| p.lost) => {
                break;
            },
        }
    })
}

