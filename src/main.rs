use std::{env::args, io::stdout};
use crossterm::{
    cursor::{Hide, MoveTo, Show}, event::EventStream, execute, style::Print, terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, SetTitle}, QueueableCommand, Result
};
use event::handle_event;
use futures::{stream::StreamExt, FutureExt};
use tokio::{pin, select, time::{interval, sleep, Duration, Instant}};

use crate::display::{draw, render};
use crate::game::*;

// use crate::debug::*;

mod debug;
mod display;
mod event;
mod game;
mod tetromino;

const TARGET_FRAME_RATE: u64 = 1_000;

pub const LOCK_RESET_LIMIT: u8 = 15;
pub const LOCK_DURATION: Duration = Duration::from_millis(500);

async fn run(game: &mut Game) -> Result<()> {
    let mut reader = EventStream::new();
    let mut stdout = stdout();

    draw()?;

    let mut render_interval = interval(Duration::from_nanos(1_000_000_000 / TARGET_FRAME_RATE));

    let drop_rate = (0.8 - (game.level - 1) as f32 * 0.007).powf((game.level - 1) as f32);
    let mut drop_interval = interval(Duration::from_nanos((drop_rate * 1_000_000_000f32) as u64));

    pin! {
        let lock_delay = sleep(Duration::ZERO);
    }

    let mut debug_interval = interval(Duration::from_secs(1));
    let mut debug_frame = 0u64;

    Ok(loop {
        select! {
            Some(event) = reader.next().fuse() => {
                match event {
                    Ok(event) => handle_event(game, event, &mut lock_delay)?,
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
                render(game)?;
                debug_frame += 1;
            },
            _ = debug_interval.tick() => {
                stdout
                    .queue(MoveTo(0, 0))?
                    .queue(Print(format!("{} fps", debug_frame)))?;
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
