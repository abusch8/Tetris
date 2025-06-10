use std::io::{stdout, Result};
use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, SetTitle},
};
use clap::Parser;
use debug::DebugWindow;

use crate::run::run;

mod debug;
mod config;
mod conn;
mod display;
mod event;
mod game;
mod player;
mod run;
mod tetromino;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    host: bool,
    #[arg(long)]
    join: bool,
    /// [default: 0.0.0.0:12000]
    #[arg(long)]
    bind_addr: Option<String>,
    /// [default: 0.0.0.0:12000]
    #[arg(long)]
    conn_addr: Option<String>,
    #[arg(long, default_value_t = 1)]
    start_level: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut stdout = stdout();

    let cli = Cli::parse();

    // let debug_window = DebugWindow::new();

    // debug_println!("peer:{} bind:{}", *config::CONN_ADDR, *config::BIND_ADDR);

    enable_raw_mode()?;
    execute!(stdout, Hide, Clear(ClearType::All), SetTitle("TETRIS"))?;

    let is_multiplayer = cli.host || cli.join;
    let is_host = cli.host;
    let start_level = cli.start_level;

    run(is_multiplayer, is_host, start_level).await?;

    execute!(stdout, Show, Clear(ClearType::All))?;
    disable_raw_mode()?;

    // println!("SCORE: {}\nLEVEL: {}\nLINES: {}", game.player[0].score, game.player[0].level, game.player[0].lines);

    // debug_window.close();

    Ok(())
}

