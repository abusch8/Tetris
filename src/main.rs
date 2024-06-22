use std::{env::args, io::{stdout, Result}};
use crossterm::{
    cursor::{Hide, Show},
    event::EventStream,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, SetTitle},
    execute,
};
use display::Display;
use event::handle_event;
use futures::{stream::StreamExt, FutureExt};
use tokio::{pin, select, time::{interval, sleep, Duration, Instant}};

use crate::game::*;
// use crate::debug::*;

mod debug;
mod config;
mod display;
mod event;
mod game;
mod tetromino;

pub const LOCK_RESET_LIMIT: u8 = 15;
pub const LOCK_DURATION: Duration = Duration::from_millis(500);

async fn run(game: &mut Game) -> Result<()> {
    let mut reader = EventStream::new();

    let display = &mut Display::new()?;

    display.draw()?;

    let frame_duration = Duration::from_nanos(if *config::MAX_FRAME_RATE > 0 {
        1_000_000_000 / *config::MAX_FRAME_RATE
    } else {
        1
    });

    let mut render_interval = interval(frame_duration);

    let drop_rate = (0.8 - (game.level - 1) as f32 * 0.007).powf((game.level - 1) as f32);
    let drop_duration = Duration::from_nanos((drop_rate * 1_000_000_000f32) as u64);

    let mut drop_interval = interval(if drop_duration.is_zero() {
        Duration::from_nanos(1)
    } else {
        drop_duration
    });

    pin! {
        let lock_delay = sleep(Duration::ZERO);
    }

    let mut debug_frame_interval = interval(Duration::from_secs(1));
    let mut debug_frame = 0u64;

    Ok(loop {
        select! {
            Some(event) = reader.next().fuse() => {
                match event {
                    Ok(event) => handle_event(event, game, display, &mut lock_delay)?,
                    Err(error) => panic!("{}", error),
                };
            },
            _ = &mut lock_delay, if game.locking => {
                game.place();
            },
            _ = drop_interval.tick() => {
                if !game.hitting_bottom(&game.falling) {
                    lock_delay.as_mut().reset(Instant::now() + LOCK_DURATION);
                }
                game.shift(ShiftDirection::Down);
            },
            _ = render_interval.tick() => {
                display.render(game)?;
                debug_frame += *config::DISPLAY_FRAME_RATE as u64;
            },
            _ = debug_frame_interval.tick(), if *config::DISPLAY_FRAME_RATE => {
                display.render_debug_info(debug_frame)?;
                debug_frame = 0;
            },
            _ = async {}, if game.end => {
                break;
            },
        };
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut stdout = stdout();

    let args = args().collect::<Vec<String>>();
    let level = if args.len() == 2 { args[1].parse::<u32>().unwrap() } else { 1 };

    // let debug_window = DebugWindow::new();

    enable_raw_mode()?;
    execute!(stdout, Hide, Clear(ClearType::All), SetTitle("TETRIS"))?;

    let game = &mut Game::start(level);
    run(game).await?;

    disable_raw_mode()?;
    execute!(stdout, Show, Clear(ClearType::All))?;
    println!("SCORE: {}\nLEVEL: {}\nLINES: {}", game.score, game.level, game.lines);

    // debug_window.close();

    Ok(())
}
