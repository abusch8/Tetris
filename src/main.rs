use std::{env::args, io::stdout, pin::Pin};
use crossterm::{
    Result, execute,
    cursor::{Hide, Show},
    terminal::{Clear, ClearType, SetTitle, enable_raw_mode, disable_raw_mode},
    event::{Event, KeyCode, EventStream},
};
use futures::{stream::StreamExt, FutureExt};
use tokio::{pin, select, time::{interval, sleep, Duration, Instant, Sleep}};

use crate::display::{draw, render};
use crate::game::*;

use crate::debug::*;

mod debug;
mod display;
mod game;
mod tetromino;

const TARGET_FRAME_RATE: u64 = 120;
const LOCK_RESET_LIMIT: u8 = 15;

fn reset_lock_timer(game: &Game, lock_delay: &mut Pin<&mut Sleep>) {
    if game.lock_reset_count < LOCK_RESET_LIMIT {
        lock_delay.as_mut().reset(Instant::now() + Duration::from_millis(500));
    }
}

fn event_handler(game: &mut Game, event: Event, lock_delay: &mut Pin<&mut Sleep>) -> Result<()> {
    Ok(match event {
        Event::Key(key) => {
            match key.code {
                KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => {
                    game.rotate(RotationDirection::Clockwise);
                    if game.locking {
                        reset_lock_timer(&game, lock_delay);
                    }
                },
                KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => {
                    game.shift(ShiftDirection::Left);
                    if game.locking && !game.hitting_left(&game.falling) {
                        reset_lock_timer(&game, lock_delay);
                    }
                },
                KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
                    if !game.hitting_bottom(&game.falling) {
                        lock_delay.as_mut().reset(Instant::now() + Duration::from_millis(500));
                    }
                    game.soft_drop();
                },
                KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => {
                    game.shift(ShiftDirection::Right);
                    if game.locking && !game.hitting_right(&game.falling) {
                        reset_lock_timer(&game, lock_delay);
                    }
                },
                KeyCode::Char(' ') => {
                    game.hard_drop();
                },
                KeyCode::Char('z') | KeyCode::Char('Z') => {
                    game.rotate(RotationDirection::CounterClockwise);
                },
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    game.hold();
                },
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    game.end = true;
                },
                _ => (),
            }
        },
        Event::Resize(_, _) => draw()?,
        _ => (),
    })
}

async fn run(game: &mut Game) -> Result<()> {
    let mut reader = EventStream::new();

    draw()?;

    let mut render_interval = interval(Duration::from_nanos(1_000_000_000 / TARGET_FRAME_RATE));
    let mut drop_interval = interval(Duration::from_millis(1_000 / game.level as u64));
    // let mut debug_interval = interval(Duration::from_secs(1));

    pin! {
        let lock_delay = sleep(Duration::ZERO);
    }

    // let mut lock_reset_count = 0u8;

    // debug_println!("{}", lock_delay.is_elapsed());

    // let mut debug_lock_start = Instant::now();
    // let mut debug_frame = 0u64;

    Ok(loop {
        select! {
            Some(event) = reader.next().fuse() => {
                match event {
                    Ok(event) => event_handler(game, event, &mut lock_delay)?,
                    Err(error) => panic!("{}", error),
                };
            },
            _ = &mut lock_delay, if game.locking => {
                // debug_println!("{} {}", debug_lock_start.elapsed().as_millis(), lock_reset_count);
                game.place();
                // lock_reset_count = 0;
            },
            _ = drop_interval.tick() => {
                if !game.hitting_bottom(&game.falling) {
                    lock_delay.as_mut().reset(Instant::now() + Duration::from_millis(500));
                    // lock_reset_count = 0;
                    // debug_lock_start = Instant::now();
                }
                game.shift(ShiftDirection::Down);
            },
            _ = render_interval.tick() => {
                render(game)?;
                // debug_frame += 1;
            },
            // _ = debug_interval.tick() => {
            //     debug_println!("FPS: {}", debug_frame);
            //     debug_frame = 0;
            // },
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

    let debug_window = DebugWindow::new();

    enable_raw_mode()?;
    execute!(stdout, Hide, Clear(ClearType::All), SetTitle("TETRIS"))?;

    let game = &mut Game::start(level);
    run(game).await?;

    disable_raw_mode()?;
    execute!(stdout, Show, Clear(ClearType::All))?;
    println!("SCORE: {}\nLEVEL: {}\nLINES: {}", game.score, game.level, game.lines);

    debug_window.close();

    Ok(())
}
