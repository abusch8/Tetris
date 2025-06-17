use std::{cmp::min, collections::HashMap, io::{stdout, Result, Stdout, Write}, time::Duration};
use crossterm::{
    cursor::MoveTo, style::{Attribute, Color, ContentStyle, StyledContent, Stylize}, terminal, QueueableCommand
};
use tokio::time::{interval, Interval};
use std::fmt::Write as FmtWrite;

use crate::{config, debug, game::Game};

pub type Dimension = (i32, i32);

pub const BOARD_DIMENSION: Dimension = (10, 20);
pub const BOARD_MP_OFFSET: u16 = 30;

pub struct Display {
    pub stdout: Stdout,
    pub render_interval: Interval,
    pub frame_count_interval: Interval,
    pub frame_count: u64,
    pub fps: u64,
    pub terminal_size: (u16, u16),
    pub board_x: Vec<(u16, u16)>,
    pub board_y: (u16, u16),
    pub is_multiplayer: bool,
}

struct TextOverlay {
    x: u16,
    y: u16,
    content: String,
    style: ContentStyle,
}

impl Display {

    fn plot_text_overlays(text_overlays: Vec<TextOverlay>) -> HashMap<(u16, u16), StyledContent<char>> {
        let mut text_map = HashMap::new();
        for TextOverlay { x, y, content, style } in text_overlays {
            for (i, c) in content.chars().enumerate() {
                text_map.insert((x + i as u16, y), StyledContent::new(style, c));
            }
        }
        return text_map;
    }

    fn init_text_overlays(&self, game: &Game, rtt: u128) -> HashMap<(u16, u16), StyledContent<char>> {
        let mut text_overlays = Vec::new();

        for (i, board_x) in self.board_x.iter().enumerate() {
            text_overlays.extend([
                TextOverlay {
                    x: board_x.0 + (board_x.1 - board_x.0) / 2 - 3,
                    y: 0,
                    content: String::from("TETRIS"),
                    style: ContentStyle::new().attribute(Attribute::Bold)
                },
                TextOverlay {
                    x: board_x.1 + 1,
                    y: 2,
                    content: String::from("NEXT:"),
                    style: ContentStyle::new(),
                },
                TextOverlay {
                    x: board_x.0 - 9,
                    y: 2,
                    content: String::from("HOLD:"),
                    style: ContentStyle::new(),
                },
                TextOverlay {
                    x: board_x.1 + 1,
                    y: 17,
                    content: format!("SCORE: {}", game.players[i].score.score),
                    style: ContentStyle::new(),
                },
                TextOverlay {
                    x: board_x.1 + 1,
                    y: 18,
                    content: format!("LEVEL: {}", game.players[i].score.level),
                    style: ContentStyle::new(),
                },
                TextOverlay {
                    x: board_x.1 + 1,
                    y: 19,
                    content: format!("LINES: {}", game.players[i].score.lines),
                    style: ContentStyle::new(),
                },
            ])
        }
        text_overlays.extend([
            TextOverlay {
                x: 0,
                y: 0,
                content: format!("{} fps", self.fps),
                style: ContentStyle::new(),
            },
            TextOverlay {
                x: 0,
                y: 1,
                content: format!("{} ms", rtt),
                style: ContentStyle::new(),
            },
        ]);
        if let Some(debugger) = &*debug::DEBUGGER.read().unwrap() {
            for i in 0..min(6, debugger.log.len()) {
                text_overlays.push(TextOverlay {
                    x: 0,
                    y: self.terminal_size.1 - i as u16 - 1,
                    content: debugger.log[debugger.log.len() - i - 1].to_string(),
                    style: ContentStyle::new(),
                });
            }
        }

        Display::plot_text_overlays(text_overlays)
    }

    fn calc_board_x(is_multiplayer: bool) -> Vec<(u16, u16)> {
        let terminal_size = terminal::size().unwrap();

        let board_x_0 = terminal_size.0 / 2 - BOARD_DIMENSION.0 as u16 * 2 / 2;
        let board_x_1 = terminal_size.0 / 2 - BOARD_DIMENSION.0 as u16 + BOARD_DIMENSION.0 as u16 * 2 + 2;

        if is_multiplayer {
            vec![
                (board_x_0 - BOARD_MP_OFFSET, board_x_1 - BOARD_MP_OFFSET),
                (board_x_0 + BOARD_MP_OFFSET, board_x_1 + BOARD_MP_OFFSET),
            ]
        } else {
            vec![
                (board_x_0, board_x_1),
            ]
        }
    }

    pub fn new(is_multiplayer: bool) -> Result<Self> {
        let stdout = stdout();

        let terminal_size = terminal::size()?;

        let board_x = Display::calc_board_x(is_multiplayer);
        let board_y = (
            0,
            BOARD_DIMENSION.1 as u16 + 2,
        );

        let max_frame_rate = *config::MAX_FRAME_RATE;
        let frame_duration = Duration::from_nanos(if max_frame_rate > 0 {
            1_000_000_000 / max_frame_rate
        } else {
            1
        });

        let render_interval = interval(frame_duration);
        let frame_count_interval = interval(Duration::from_secs(1));

        Ok(Display {
            stdout,
            render_interval,
            frame_count_interval,
            frame_count: 0u64,
            fps: 0u64,
            terminal_size,
            board_x,
            board_y,
            is_multiplayer,
        })
    }

    pub fn resize(&mut self) -> Result<()> {
        self.terminal_size = terminal::size()?;
        self.board_x = Display::calc_board_x(self.is_multiplayer);
        Ok(())
    }

    pub fn construct_frame(&self, game: &Game, rtt: u128) -> Result<Vec<Vec<StyledContent<char>>>> {
        let text_map = self.init_text_overlays(game, rtt);
        let width = self.terminal_size.0 as usize;
        let height = self.terminal_size.1 as usize;
        let mut frame = Vec::with_capacity(height);
        for y in 0..height {
            let mut row = Vec::with_capacity(width);
            for x in 0..width {
                let cell = text_map
                    .get(&(x as u16, y as u16))
                    .cloned()
                    .or(self.render_tetromino(game, x as u16, y as u16).ok().flatten())
                    .or(self.render_board(x as u16, y as u16).ok().flatten())
                    .unwrap_or(StyledContent::new(ContentStyle::new(), ' '));
                row.push(cell);
            }
            frame.push(row);
        }
        Ok(frame)
    }

    pub fn render(&mut self, game: &Game, rtt: u128) -> Result<()> {
        let frame = self.construct_frame(game, rtt)?;

        // let mut buf = String::new();
        // for (i, row) in frame.iter().enumerate() {
        //     stdout().queue(MoveTo(0, i as u16))?;
        //
        //     // let mut line = String::new();
        //     use std::fmt::Write as FmtWrite;
        //     write!(&mut buf, "\r\n").unwrap();
        //     for cell in row {
        //         write!(&mut buf, "{}", cell).unwrap(); // includes styling
        //     }
        //
        // }
        // stdout().write_all(buf.as_bytes())?;

        for (i, row) in frame.iter().enumerate() {
            self.stdout.queue(MoveTo(0, i as u16))?;

            let mut buf = String::new();
            for cell in row {
                write!(&mut buf, "{}", cell).unwrap();
            }
            self.stdout.write_all(buf.as_bytes())?;
        }

        self.frame_count += *config::DISPLAY_FRAME_RATE as u64;
        Ok(())
    }

    fn render_board(&self, x: u16, y: u16) -> Result<Option<StyledContent<char>>> {
        let mut content = None;

        for board_x in self.board_x.iter() {
            if x >= board_x.0 && x <= board_x.1 - 1 && y >= self.board_y.0 && y <= self.board_y.1 - 1 {
                content = Some(StyledContent::new(ContentStyle::new(),
                    if x == board_x.0 && y == 0 {
                        '╔'
                    } else if x == board_x.0 && y == self.board_y.1 - 1 {
                        '╚'
                    } else if x == board_x.1 - 1 && y == self.board_y.0 {
                        '╗'
                    } else if x == board_x.1 - 1 && y == self.board_y.1 - 1 {
                        '╝'
                    } else if x == board_x.0 || x == board_x.1 - 1 {
                        '║'
                    } else if y == self.board_y.0 || y == self.board_y.1 - 1 {
                        '═'
                    } else if x % 2 != self.terminal_size.0 / 2 % 2 {
                        '.'
                    } else {
                        ' '
                    }
                ));
            }
        }

        Ok(content)
    }

    fn render_tetromino(&self, game: &Game, x: u16, y: u16) -> Result<Option<StyledContent<char>>> {
        let mut content = None;

        for (i, board_x) in self.board_x.iter().enumerate() {
            if x > board_x.0 && x < board_x.1 - 1 && y > self.board_y.0 && y < self.board_y.1 - 1 {
                if let Some(ghost) = &game.players[i].ghost {
                    if ghost.at_pos(x, self.board_y.1 - y - 2, board_x.0, 0) {
                        content = Some('░'.with(game.players[i].falling.color));
                    }
                }

                if game.players[i].falling.at_pos(x, self.board_y.1 - y - 2, board_x.0, 0) {
                    content = Some(if game.players[i].locking {
                        '▓'.with(game.players[i].falling.color)
                    } else {
                        ' '.on(game.players[i].falling.color)
                    });
                }

                let j = (self.board_y.1 - 2 - y) as usize;
                let k = ((x - board_x.0 - 1) / 2) as usize;

                if let Some(color) = game.players[i].stack[j][k] {
                    content = Some(if game.players[i].clearing.get(&j).is_some() {
                        '▓'.with(Color::White)
                    } else {
                        ' '.on(color)
                    })
                }

            }
            if let Some(holding) = &game.players[i].holding {
                if holding.at_pos(x, y, board_x.0 - 11, 4) {
                    content = Some(' '.on(holding.color));
                }
            }
            for (i, next) in game.players[i].bag.next.iter().enumerate() {
                if next.at_pos(x, y, board_x.1, ((i as u16 + 1) * 3) + 1) {
                    content = Some(' '.on(next.color));
                }
            }
        }

        Ok(content)
    }

    pub fn calc_fps(&mut self) {
        self.fps = self.frame_count;
        self.frame_count = 0;
    }
}

