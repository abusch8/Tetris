use std::{env::args, io::{stdout, Result}};
use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, SetTitle},
};
use debug::DebugWindow;

use crate::{game::Game, run::run};

mod debug;
mod config;
mod display;
mod event;
mod game;
mod run;
mod tetromino;

#[tokio::main]
async fn main() -> Result<()> {
    let mut stdout = stdout();

    let debug_window = DebugWindow::new();

    // debug_println!("peer:{} bind:{}", *config::CONN_ADDR, *config::BIND_ADDR);

    let args = args().collect::<Vec<String>>();
    let level = 1; // if args.len() == 2 { args[1].parse::<u32>().unwrap() } else { 1 };
    let is_host = args.len() == 2 && args[1] == "host";

    enable_raw_mode()?;
    execute!(stdout, Hide, Clear(ClearType::All), SetTitle("TETRIS"))?;

    let game = &mut Game::start(level);
    run(game, is_host).await?;

    execute!(stdout, Show, Clear(ClearType::All))?;
    disable_raw_mode()?;

    println!("SCORE: {}\nLEVEL: {}\nLINES: {}", game.score, game.level, game.lines);

    debug_window.close();

    Ok(())
}

