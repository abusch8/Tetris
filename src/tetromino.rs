use crossterm::style::Color;
use num_derive::FromPrimitive;
use strum_macros::EnumIter;

use crate::display::Dimension;

const USE_XTERM_256_COLORS: bool = true;

pub type Shape = Vec<Dimension>;

#[derive(Clone, Copy, FromPrimitive, PartialEq)]
pub enum CardinalDirection { North, East, South, West }

#[derive(Clone, Copy, EnumIter, FromPrimitive, PartialEq)]
pub enum TetrominoVariant { I, J, L, O, S, T, Z }

#[derive(Clone)]
pub struct Tetromino {
    pub shape: Shape,
    pub center: Dimension,
    pub direction: CardinalDirection,
    pub color: Color,
    pub variant: TetrominoVariant,
}

impl Tetromino {
    pub fn new(variant: TetrominoVariant) -> Self {
        match variant {
            TetrominoVariant::I => Tetromino {
                shape: vec![(3, 18), (4, 18), (5, 18), (6, 18)],
                center: (4, 18),
                direction: CardinalDirection::North,
                color: if USE_XTERM_256_COLORS { Color::AnsiValue(51) } else { Color::Cyan },
                variant,
            },
            TetrominoVariant::J => Tetromino {
                shape: vec![(4, 19), (4, 18), (5, 18), (6, 18)],
                center: (5, 18),
                direction: CardinalDirection::North,
                color: if USE_XTERM_256_COLORS { Color::AnsiValue(33) } else { Color::Blue },
                variant,
            },
            TetrominoVariant::L => Tetromino {
                shape: vec![(4, 18), (5, 18), (6, 18), (6, 19)],
                center: (5, 18),
                direction: CardinalDirection::North,
                color: if USE_XTERM_256_COLORS { Color::AnsiValue(202) } else { Color::White },
                variant,
            },
            TetrominoVariant::O => Tetromino {
                shape: vec![(4, 18), (4, 19), (5, 18), (5, 19)],
                center: (4, 18),
                direction: CardinalDirection::North,
                color: if USE_XTERM_256_COLORS { Color::AnsiValue(226) } else { Color::Yellow },
                variant,
            },
            TetrominoVariant::S => Tetromino {
                shape: vec![(4, 18), (5, 18), (5, 19), (6, 19)],
                center: (5, 18),
                direction: CardinalDirection::North,
                color: if USE_XTERM_256_COLORS { Color::AnsiValue(40) } else { Color::Green },
                variant,
            },
            TetrominoVariant::T => Tetromino {
                shape: vec![(4, 18), (5, 18), (5, 19), (6, 18)],
                center: (5, 18),
                direction: CardinalDirection::North,
                color: if USE_XTERM_256_COLORS { Color::AnsiValue(165) } else { Color::Magenta },
                variant,
            },
            TetrominoVariant::Z => Tetromino {
                shape: vec![(4, 19), (5, 19), (5, 18), (6, 18)],
                center: (5, 18),
                direction: CardinalDirection::North,
                color: if USE_XTERM_256_COLORS { Color::AnsiValue(196) } else { Color::Red },
                variant,
            },
        }
    }
}
