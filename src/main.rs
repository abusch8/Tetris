use std::{io::stdout, time::{Instant, Duration}, env::args, thread::sleep};
use crossterm::{
    Result, execute,
    cursor::{Hide, Show},
    terminal::{Clear, ClearType, SetTitle, enable_raw_mode, disable_raw_mode},
    event::{Event, KeyCode, read, poll},
};

use crate::display::{draw, render};
use crate::game::*;

use crate::debug::*;

mod debug;
mod display;
mod game;
mod tetromino;

fn main() -> Result<()> {
    let mut stdout = stdout();

    let debug_window = DebugWindow::new();

    let args = args().collect::<Vec<String>>();
    let level = if args.len() == 2 { args[1].parse::<u32>().unwrap() } else { 1 };

    enable_raw_mode()?;

    execute!(stdout, Hide, Clear(ClearType::All), SetTitle("TETRIS"))?;

    let game = &mut Game::start(level);

    draw(game)?;

    let tick_frequency = game.level * 2;

    let tick_duration = Duration::from_secs(1) / tick_frequency;
    let lock_delay_duration = Duration::from_millis(500);

    let mut tick_start = Instant::now();
    let mut lock_delay_start: Option<Instant> = None;

    macro_rules! quit {
        () => {{
            debug_window.close();
            disable_raw_mode()?;
            execute!(stdout, Show, Clear(ClearType::All))?;
            println!("SCORE: {}\nLEVEL: {}\nLINES: {}", game.score, game.level, game.lines);
            break
        }};
    }

    Ok(loop {
        if game.end { quit!() }

        if game.locking {
            match lock_delay_start {
                Some(remaining_duration) => {
                    if lock_delay_duration.checked_sub(remaining_duration.elapsed()).is_none() {
                        game.place();
                        game.locking = false;
                    }
                },
                None => lock_delay_start = Some(Instant::now()),
            }
        } else {
            lock_delay_start = None;
        }

        match tick_duration.checked_sub(tick_start.elapsed()) {
            Some(remaining_duration) => {
                if poll(remaining_duration)? {
                    match read()? {
                        Event::Resize(_, _) => draw(game)?,
                        Event::Key(event) => match event.code {
                            KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => game.rotate(RotationDirection::Clockwise),
                            KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => game.shift(ShiftDirection::Left),
                            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => game.soft_drop(),
                            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => game.shift(ShiftDirection::Right),
                            KeyCode::Char('z') | KeyCode::Char('Z') => game.rotate(RotationDirection::CounterClockwise),
                            KeyCode::Char('c') | KeyCode::Char('C') => game.hold(),
                            KeyCode::Char(' ') => game.hard_drop(),
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => quit!(),
                            _ => continue,
                        },
                        _ => continue,
                    }
                }
            },
            None => {
                game.shift(ShiftDirection::Down);
                tick_start = Instant::now();
            },
        }
        render(game)?;
        sleep(Duration::from_millis(1));
    })
}
