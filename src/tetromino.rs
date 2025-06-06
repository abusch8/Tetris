use crossterm::style::Color;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use strum_macros::EnumIter;

use crate::{config, display::Dimension, player::RotationDirection};

pub type Shape = Vec<Dimension>;

#[derive(Clone, Copy, FromPrimitive, ToPrimitive, PartialEq, Debug)]
pub enum CardinalDirection { North, East, South, West }

#[derive(Clone, Copy, EnumIter, FromPrimitive, ToPrimitive, PartialEq, Debug)]
pub enum TetrominoVariant { I, J, L, O, S, T, Z }

#[derive(Clone, PartialEq, Debug)]
pub struct Geometry {
    pub shape: Shape,
    pub center: Dimension,
    pub direction: CardinalDirection,
}

impl Geometry {
    pub fn transform(&mut self, x: i32, y: i32) {
        for shape in &mut self.shape {
            shape.0 += x;
            shape.1 += y;
        }
        self.center.0 += x;
        self.center.1 += y;
    }

    pub fn rotate(&mut self, direction: RotationDirection) {
        let (angle, new_direction) = match direction {
            RotationDirection::Clockwise => (
                f32::from(-90.0).to_radians(),
                CardinalDirection::from_i32((self.direction as i32 + 1) % 4).unwrap(),
            ),
            RotationDirection::CounterClockwise => (
                f32::from(90.0).to_radians(),
                CardinalDirection::from_i32(((self.direction as i32 - 1) % 4 + 4) % 4).unwrap(),
            ),
        };

        for shape in &mut self.shape {
            let x = (shape.0 - self.center.0) as f32;
            let y = (shape.1 - self.center.1) as f32;
            shape.0 = ((x * angle.cos() - y * angle.sin()) + self.center.0 as f32).round() as i32;
            shape.1 = ((x * angle.sin() + y * angle.cos()) + self.center.1 as f32).round() as i32;
        }

        self.direction = new_direction;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.extend_from_slice(&self.direction.to_u8().unwrap().to_le_bytes());
        buf.extend_from_slice(&self.center.0.to_le_bytes());
        buf.extend_from_slice(&self.center.1.to_le_bytes());

        for (x, y) in &self.shape {
            buf.extend_from_slice(&x.to_le_bytes());
            buf.extend_from_slice(&y.to_le_bytes());
        }

        buf
    }

    pub fn from_bytes(buf: &[u8; 41]) -> Geometry {
        let direction_bytes: &[u8; 1] = buf[0..1].try_into().unwrap();
        let direction = CardinalDirection::from_u8(u8::from_le_bytes(*direction_bytes)).unwrap();

        let center_x_bytes: &[u8; 4] = buf[1..5].try_into().unwrap();
        let center_y_bytes: &[u8; 4] = buf[5..9].try_into().unwrap();
        let center = (
            i32::from_le_bytes(*center_x_bytes),
            i32::from_le_bytes(*center_y_bytes),
        );

        let mut shape = Vec::new();
        for i in (9..buf.len()).step_by(8) {
            let shape_x_bytes: &[u8; 4] = buf[i..i + 4].try_into().unwrap();
            let shape_y_bytes: &[u8; 4] = buf[i + 4..i + 8].try_into().unwrap();
            shape.push((
                i32::from_le_bytes(*shape_x_bytes),
                i32::from_le_bytes(*shape_y_bytes),
            ));
        }

        Geometry {
            shape, center, direction
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Tetromino {
    pub geometry: Geometry,
    pub color: Color,
    pub variant: TetrominoVariant,
}

impl Tetromino {
    pub fn new(variant: TetrominoVariant) -> Self {
        match variant {
            TetrominoVariant::I => Tetromino {
                geometry: Geometry {
                    shape: vec![(3, 18), (4, 18), (5, 18), (6, 18)],
                    center: (4, 18),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(51) } else { Color::Cyan },
                variant,
            },
            TetrominoVariant::J => Tetromino {
                geometry: Geometry {
                    shape: vec![(4, 19), (4, 18), (5, 18), (6, 18)],
                    center: (5, 18),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(33) } else { Color::Blue },
                variant,
            },
            TetrominoVariant::L => Tetromino {
                geometry: Geometry {
                    shape: vec![(4, 18), (5, 18), (6, 18), (6, 19)],
                    center: (5, 18),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(202) } else { Color::White },
                variant,
            },
            TetrominoVariant::O => Tetromino {
                geometry: Geometry {
                    shape: vec![(4, 18), (4, 19), (5, 18), (5, 19)],
                    center: (4, 18),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(226) } else { Color::Yellow },
                variant,
            },
            TetrominoVariant::S => Tetromino {
                geometry: Geometry {
                    shape: vec![(4, 18), (5, 18), (5, 19), (6, 19)],
                    center: (5, 18),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(40) } else { Color::Green },
                variant,
            },
            TetrominoVariant::T => Tetromino {
                geometry: Geometry {
                    shape: vec![(4, 18), (5, 18), (5, 19), (6, 18)],
                    center: (5, 18),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(165) } else { Color::Magenta },
                variant,
            },
            TetrominoVariant::Z => Tetromino {
                geometry: Geometry {
                    shape: vec![(4, 19), (5, 19), (5, 18), (6, 18)],
                    center: (5, 18),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(196) } else { Color::Red },
                variant,
            },
        }
    }
}

