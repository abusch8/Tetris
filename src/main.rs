use std::{io::stdout, env::args};
use crossterm::{
    Result, execute,
    cursor::{Hide, Show},
    terminal::{Clear, ClearType, SetTitle, enable_raw_mode, disable_raw_mode},
    event::{Event, KeyCode, EventStream},
};
use futures::{FutureExt, stream::StreamExt};
use tokio::{select, time::{interval, Duration, sleep_until, Instant}, pin};

use crate::display::{draw, render};
use crate::game::*;

use crate::debug::*;

mod debug;
mod display;
mod game;
mod tetromino;

const TARGET_FRAME_RATE: u64 = 120;

async fn run(game: &mut Game) {
    let mut reader = EventStream::new();

    draw().unwrap();

    let mut render_interval = interval(Duration::from_nanos(1_000_000_000 / TARGET_FRAME_RATE));
    let mut drop_interval = interval(Duration::from_millis(1_000 / game.level as u64));
    let mut debug_interval = interval(Duration::from_secs(1));

    let lock_delay = sleep_until(Instant::now());
    pin!(lock_delay);

    let mut frame = 0;

    loop {
        select! {
            maybe_event = reader.next().fuse() => {
                match maybe_event {
                    Some(Ok(event)) => {
                        if let Event::Key(key) = event {
                            match key.code {
                                KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => {
                                    game.rotate(RotationDirection::Clockwise)
                                },
                                KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => {
                                    game.shift(ShiftDirection::Left)
                                },
                                KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
                                    game.soft_drop()
                                },
                                KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => {
                                    game.shift(ShiftDirection::Right)
                                },
                                KeyCode::Char(' ') => {
                                    game.hard_drop()
                                },
                                KeyCode::Char('z') | KeyCode::Char('Z') => {
                                    game.rotate(RotationDirection::CounterClockwise)
                                },
                                KeyCode::Char('c') | KeyCode::Char('C') => {
                                    game.hold()
                                },
                                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                                    break
                                },
                                _ => (),
                            }
                        }
                    },
                    Some(Err(error)) => panic!("{}", error),
                    None => (),
                }
            },
            _ = async {}, if game.lock_reset || game.locking && lock_delay.is_elapsed() => {
                lock_delay.as_mut().reset(Instant::now() + Duration::from_millis(500));
            },
            _ = &mut lock_delay, if game.locking => {
                game.place();
            },
            _ = drop_interval.tick() => {
                game.shift(ShiftDirection::Down)
            },
            _ = render_interval.tick() => {
                render(game).unwrap();
                frame += 1;
            },
            _ = debug_interval.tick() => {
                debug_println!("FPS: {}", frame);
                frame = 0;
            },
        };
    }
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
    run(game).await;

    disable_raw_mode()?;
    execute!(stdout, Show, Clear(ClearType::All))?;
    println!("SCORE: {}\nLEVEL: {}\nLINES: {}", game.score, game.level, game.lines);

    debug_window.close();

    Ok(())
}
