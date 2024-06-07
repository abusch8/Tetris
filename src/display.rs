use::std::io::{stdout, Write};
use crossterm::{
    cursor::MoveTo, execute, style::{ContentStyle, Print, PrintStyledContent, StyledContent, Stylize}, terminal::{self, Clear, ClearType}, QueueableCommand, Result
};

use crate::game::Game;

use crate::debug_println;

pub const BOARD_DIMENSION: Dimension = (10, 20);

pub type Dimension = (i32, i32);

const BOARD_WIDTH: u16 = BOARD_DIMENSION.0 as u16 * 2 + 2;
const BOARD_HEIGHT: u16 = BOARD_DIMENSION.1 as u16 + 2;

pub fn render(game: &Game) -> Result<()> {
    let mut stdout = stdout();

    debug_println!("RERENDER");

    let (terminal_width, _terminal_height) = terminal::size().unwrap();

    let board_width_start = terminal_width / 2 - BOARD_WIDTH / 2;
    let board_width_end = board_width_start + BOARD_WIDTH;

    fn position_in_view(position: &Dimension, view: &Dimension, offset_x: i32) -> bool {
        BOARD_DIMENSION.1 - position.1 == view.1 && ((position.0 + 1) * 2 + offset_x == view.0 || (position.0 + 1) * 2 + offset_x - 1 == view.0)
    }

    for x in board_width_start + 1..board_width_end - 1 {
        for y in 1..BOARD_HEIGHT - 1 {
            stdout
                .queue(MoveTo(x as u16, y as u16))?
                .queue(PrintStyledContent((|| {
                    if game.falling.shape.iter().any(|position| position_in_view(position, &(x as i32, y as i32), board_width_start as i32)) {
                        return if game.locking { "▓".with(game.falling.color) } else { " ".on(game.falling.color) }
                    }
                    if let Some(ghost) = &game.ghost {
                        if ghost.shape.iter().any(|position| position_in_view(position, &(x as i32, y as i32), board_width_start as i32)) {
                            return "░".with(game.falling.color)
                        }
                    }
                    for (i, row) in game.stack.iter().enumerate() {
                        for (j, color) in row.iter().enumerate() {
                            if let Some(color) = color {
                                if position_in_view(&(j as i32, i as i32), &(x as i32, y as i32), board_width_start as i32) {
                                    return " ".on(*color)
                                }
                            }
                        }
                    }
                    StyledContent::new(ContentStyle::new(), if x % 2 != terminal_width / 2 % 2 { "." } else { " " })
                })()))?;
        }
    }
    for (i, tetromino) in game.next.iter().enumerate() {
        stdout
            .queue(MoveTo(board_width_end + 1, 4 + (i as u16 * 3)))?
            .queue(Print("        "))?
            .queue(MoveTo(board_width_end + 1, 5 + (i as u16 * 3)))?
            .queue(Print("        "))?;
        for position in tetromino.shape.iter().map(|(x, y)| (*x as u16, *y as u16)) {
            stdout
                .queue(MoveTo((position.0 - 3) * 2 + board_width_end + 2, BOARD_HEIGHT - position.1 + 1 + (i as u16 * 3)))?
                .queue(PrintStyledContent(" ".on(tetromino.color)))?
                .queue(MoveTo((position.0 - 3) * 2 + board_width_end + 1, BOARD_HEIGHT - position.1 + 1 + (i as u16 * 3)))?
                .queue(PrintStyledContent(" ".on(tetromino.color)))?;
        }
    }
    if let Some(holding) = &game.holding {
        stdout
            .queue(MoveTo(board_width_start - 9, 4))?
            .queue(Print("        "))?
            .queue(MoveTo(board_width_start - 9, 5))?
            .queue(Print("        "))?;
        for position in holding.shape.iter().map(|(x, y)| (*x as u16, *y as u16)) {
            stdout
                .queue(MoveTo((position.0 - 3) * 2 + board_width_start - 9, BOARD_HEIGHT - position.1 + 1))?
                .queue(PrintStyledContent(" ".on(holding.color)))?
                .queue(MoveTo((position.0 - 3) * 2 + board_width_start - 8, BOARD_HEIGHT - position.1 + 1))?
                .queue(PrintStyledContent(" ".on(holding.color)))?;
        }
    }
    stdout
        .queue(MoveTo(board_width_end + 1, 17))?
        .queue(Print(format!("SCORE: {}", game.score)))?
        .queue(MoveTo(board_width_end + 1, 18))?
        .queue(Print(format!("LEVEL: {}", game.level)))?
        .queue(MoveTo(board_width_end + 1, 19))?
        .queue(Print(format!("LINES: {}", game.lines)))?
        .queue(MoveTo(0, 0))?
        .flush()?;

    Ok(())
}

pub fn draw() -> Result<()> {
    let mut stdout = stdout();

    execute!(stdout, Clear(ClearType::All))?;

    let (terminal_width, _terminal_height) = terminal::size().unwrap();

    // debug_println!("REDRAW");
    // debug_println!("{}", terminal_width);

    let board_width_start = terminal_width / 2 - BOARD_WIDTH / 2;
    let board_width_end = board_width_start + BOARD_WIDTH;

    // debug_println!("{}", board_width_start);

    for x in board_width_start..board_width_end {
        for y in 0..BOARD_HEIGHT {
            stdout
                .queue(MoveTo(x, y))?
                .queue(Print(
                    if x == board_width_start && y == 0 {
                        "╔"
                    } else if x == board_width_start && y == BOARD_HEIGHT - 1 {
                        "╚"
                    } else if x == board_width_end - 1 && y == 0 {
                        "╗"
                    } else if x == board_width_end - 1 && y == BOARD_HEIGHT - 1 {
                        "╝"
                    } else if x == board_width_start || x == board_width_end - 1 {
                        "║"
                    } else if y == 0 || y == BOARD_HEIGHT - 1 {
                        "═"
                    } else if x % 2 != terminal_width / 2 % 2 {
                        "."
                    } else {
                        " "
                    }
                ))?;
        }
    }
    stdout
        .queue(MoveTo(board_width_start + 8, 0))?
        .queue(PrintStyledContent("TETRIS".bold()))?
        .queue(MoveTo(board_width_end + 1, 2))?
        .queue(Print("NEXT:"))?
        .queue(MoveTo(board_width_start - 9, 2))?
        .queue(Print("HOLD:"))?
        .queue(MoveTo(0, 0))?
        .flush()?;

    Ok(())
}
