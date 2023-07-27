use::std::io::{stdout, Write};
use crossterm::{
    Result, QueueableCommand,
    style::{PrintStyledContent, StyledContent, Stylize, ContentStyle, Print},
    cursor::MoveTo,
};

use crate::game::Game;

pub type Dimension = (i32, i32);

pub const BOARD_DIMENSION: Dimension = (10, 20);

const WIDTH: u16 = BOARD_DIMENSION.0 as u16 * 2 + 2;
const HEIGHT: u16 = BOARD_DIMENSION.1 as u16 + 2;

pub fn render(game: &Game) -> Result<()> {
    let mut stdout = stdout();

    fn position_in_view(position: &Dimension, view: &Dimension) -> bool {
        BOARD_DIMENSION.1 - position.1 == view.1 && ((position.0 + 1) * 2 == view.0 || (position.0 + 1) * 2 - 1 == view.0)
    }

    for x in 1..WIDTH as i32 - 1 {
        for y in 1..HEIGHT as i32 - 1 {
            stdout
                .queue(MoveTo(x as u16, y as u16))?
                .queue(PrintStyledContent((|| {
                    if game.falling.shape.iter().any(|position| position_in_view(position, &(x, y))) {
                        return if game.locking { "▓".with(game.falling.color) } else { " ".on(game.falling.color) }
                    }
                    if let Some(ghost) = &game.ghost {
                        if ghost.shape.iter().any(|position| position_in_view(position, &(x, y))) {
                            return "░".with(game.falling.color)
                        }
                    }
                    for (i, row) in game.stack.iter().enumerate() {
                        for (j, color) in row.iter().enumerate() {
                            if let Some(color) = color {
                                if position_in_view(&(j as i32, i as i32), &(x, y)) {
                                    return " ".on(*color)
                                }
                            }
                        }
                    }
                    StyledContent::new(ContentStyle::new(), if x % 2 == 0 { "." } else { " " })
                })()))?;
        }
    }
    stdout
        .queue(MoveTo(WIDTH + 1, 4))?
        .queue(Print("        "))?
        .queue(MoveTo(WIDTH + 1, 5))?
        .queue(Print("        "))?;
    for position in game.next.shape.iter().map(|(x, y)| (*x as u16, *y as u16)) {
        stdout
            .queue(MoveTo((position.0 - 3) * 2 + WIDTH + 2, HEIGHT - position.1 + 1))?
            .queue(PrintStyledContent(" ".on(game.next.color)))?
            .queue(MoveTo((position.0 - 3) * 2 + WIDTH + 1, HEIGHT - position.1 + 1))?
            .queue(PrintStyledContent(" ".on(game.next.color)))?;
    }
    if let Some(holding) = &game.holding {
        stdout
            .queue(MoveTo(WIDTH + 1, 9))?
            .queue(Print("        "))?
            .queue(MoveTo(WIDTH + 1, 10))?
            .queue(Print("        "))?;
        for position in holding.shape.iter().map(|(x, y)| (*x as u16, *y as u16)) {
            stdout
                .queue(MoveTo((position.0 - 3) * 2 + WIDTH + 2, HEIGHT - position.1 + 6))?
                .queue(PrintStyledContent(" ".on(holding.color)))?
                .queue(MoveTo((position.0 - 3) * 2 + WIDTH + 1, HEIGHT - position.1 + 6))?
                .queue(PrintStyledContent(" ".on(holding.color)))?;
        }
    }
    stdout
        .queue(MoveTo(WIDTH + 1, 12))?
        .queue(Print(format!("SCORE: {}", game.score)))?
        .queue(MoveTo(WIDTH + 1, 13))?
        .queue(Print(format!("LEVEL: {}", game.level)))?
        .queue(MoveTo(WIDTH + 1, 14))?
        .queue(Print(format!("LINES: {}", game.lines)))?
        .queue(MoveTo(0, 0))?
        .flush()?;

    Ok(())
}

pub fn draw() -> Result<()> {
    let mut stdout = stdout();

    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            stdout
                .queue(MoveTo(x, y))?
                .queue(Print(
                    if x == 0 && y == 0 {
                        "╔"
                    } else if x == 0 && y == HEIGHT - 1 {
                        "╚"
                    } else if x == WIDTH - 1 && y == 0 {
                        "╗"
                    } else if x == WIDTH - 1 && y == HEIGHT - 1 {
                        "╝"
                    } else if x == 0 || x == WIDTH - 1 {
                        "║"
                    } else if y == 0 || y == HEIGHT - 1 {
                        "═"
                    } else if x % 2 == 0 {
                        "."
                    } else {
                        " "
                    }
                ))?;
        }
    }
    stdout
        .queue(MoveTo(8, 0))?
        .queue(PrintStyledContent("TETRIS".bold()))?
        .queue(MoveTo(WIDTH + 1, 2))?
        .queue(Print("NEXT:"))?
        .queue(MoveTo(WIDTH + 1, 7))?
        .queue(Print("HOLDING:"))?
        .queue(MoveTo(0, 0))?
        .flush()?;

    Ok(())
}
