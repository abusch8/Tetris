use std::io::{stdout, Result};
use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, SetTitle},
};
use clap::{Parser, ValueEnum};

use crate::{conn::ConnKind, run::run};

mod agent;
mod bag;
mod board;
mod color;
mod config;
mod conn;
mod debug;
mod display;
mod event;
mod game;
mod player;
mod run;
mod score;
mod tetromino;

#[derive(ValueEnum, Clone, Copy)]
pub enum Mode {
    #[clap(alias = "s")]
    Singleplayer,
    #[clap(alias = "m")]
    Multiplayer,
    #[clap(alias = "pvc")]
    PlayerVsComputer,
    #[clap(alias = "cvc")]
    ComputerVsComputer,
}

#[derive(Parser)]
#[command(version)]
pub struct Cli {
    #[arg(value_enum, default_value_t = Mode::Singleplayer)]
    mode: Mode,
    #[arg(long, short = 'H', conflicts_with = "join")]
    host: bool,
    #[arg(long, short = 'J', conflicts_with = "host")]
    join: bool,
    /// [default: 0.0.0.0:12000]
    #[arg(long)]
    bind_addr: Option<String>,
    /// [default: 0.0.0.0:12000]
    #[arg(long)]
    conn_addr: Option<String>,
    #[arg(long, default_value_t = 1)]
    start_level: u32,
    #[arg(long)]
    party: bool,
    #[arg(long, short)]
    debug: bool,
}

pub fn enter_tui_mode() -> Result<()> {
    execute!(stdout(), Hide, Clear(ClearType::All), SetTitle("TETRIS"))?;
    enable_raw_mode()?;
    Ok(())
}

pub fn exit_tui_mode() -> Result<()> {
    execute!(stdout(), Show, Clear(ClearType::All))?;
    disable_raw_mode()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    enter_tui_mode()?;

    let conn_kind = ConnKind::from_args(&args);
    let start_level = args.start_level;

    run(args.mode, conn_kind, start_level).await?;

    exit_tui_mode()?;

    Ok(())
}

