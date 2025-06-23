use std::io::{stdout, Result};
use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, SetTitle},
};
use clap::Parser;

use crate::{conn::ConnType, run::run};

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

#[derive(Parser)]
#[command(version)]
pub struct Cli {
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
    #[arg(long)]
    party: bool,
    #[arg(long)]
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
    let cli = Cli::parse();

    enter_tui_mode()?;

    let conn_kind = ConnType::from_args(cli.host, cli.join);
    let start_level = cli.start_level;

    run(conn_kind, start_level).await?;

    exit_tui_mode()?;

    Ok(())
}

