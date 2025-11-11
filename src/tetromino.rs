use crossterm::style::Color;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use strum_macros::EnumIter;

use crate::{config, board::{Board, Dimension}};

static JLSTZ_OFFSETS: [[(i32, i32); 5]; 4] = [
    [( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0)], // North
    [( 0,  0), ( 1,  0), ( 1, -1), ( 0,  2), ( 1,  2)], // East
    [( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0)], // South
    [( 0,  0), (-1,  0), (-1, -1), ( 0,  2), (-1,  2)], // West
];

static I_OFFSETS: [[(i32, i32); 5]; 4] = [
    [( 0,  0), (-1,  0), ( 2,  0), (-1,  0), ( 2,  0)],
    [(-1,  0), ( 0,  0), ( 0,  0), ( 0,  1), ( 0, -2)],
    [(-1,  1), ( 1,  1), (-2,  1), ( 1,  0), (-2,  0)],
    [( 0,  1), ( 0,  1), ( 0,  1), ( 0, -1), ( 0,  2)],
];

static O_OFFSETS: [[(i32, i32); 5]; 4] = [
    [( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0)],
    [( 0, -1), ( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0)],
    [(-1, -1), ( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0)],
    [(-1,  0), ( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0)],
];

pub type Shape = Vec<Dimension>;

#[derive(FromPrimitive, PartialEq)]
pub enum ShiftDirection { Left, Right }

#[derive(PartialEq)]
pub enum RotationDirection { Clockwise, CounterClockwise }

#[derive(Clone, Copy, EnumIter, FromPrimitive, ToPrimitive, PartialEq, Debug)]
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

    pub fn from_bytes(buf: [u8; 41]) -> Geometry {
        let direction_bytes: [u8; 1] = buf[0..1].try_into().unwrap();
        let direction = CardinalDirection::from_u8(u8::from_le_bytes(direction_bytes)).unwrap();

        let center_x_bytes: [u8; 4] = buf[1..5].try_into().unwrap();
        let center_y_bytes: [u8; 4] = buf[5..9].try_into().unwrap();
        let center = (
            i32::from_le_bytes(center_x_bytes),
            i32::from_le_bytes(center_y_bytes),
        );

        let mut shape = Vec::new();
        for i in (9..buf.len()).step_by(8) {
            let shape_x_bytes: [u8; 4] = buf[i..i + 4].try_into().unwrap();
            let shape_y_bytes: [u8; 4] = buf[i + 4..i + 8].try_into().unwrap();
            shape.push((
                i32::from_le_bytes(shape_x_bytes),
                i32::from_le_bytes(shape_y_bytes),
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
                    shape: vec![(0, 1), (1, 1), (2, 1), (3, 1)],
                    center: (1, 1),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(51) } else { Color::Cyan },
                variant,
            },
            TetrominoVariant::J => Tetromino {
                geometry: Geometry {
                    shape: vec![(1, 1), (1, 0), (2, 0), (3, 0)],
                    center: (2, 0),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(33) } else { Color::Blue },
                variant,
            },
            TetrominoVariant::L => Tetromino {
                geometry: Geometry {
                    shape: vec![(1, 0), (2, 0), (3, 0), (3, 1)],
                    center: (2, 0),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(202) } else { Color::White },
                variant,
            },
            TetrominoVariant::O => Tetromino {
                geometry: Geometry {
                    shape: vec![(1, 0), (1, 1), (2, 0), (2, 1)],
                    center: (1, 0),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(226) } else { Color::Yellow },
                variant,
            },
            TetrominoVariant::S => Tetromino {
                geometry: Geometry {
                    shape: vec![(1, 0), (2, 0), (2, 1), (3, 1)],
                    center: (2, 0),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(40) } else { Color::Green },
                variant,
            },
            TetrominoVariant::T => Tetromino {
                geometry: Geometry {
                    shape: vec![(1, 0), (2, 0), (2, 1), (3, 0)],
                    center: (2, 0),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(165) } else { Color::Magenta },
                variant,
            },
            TetrominoVariant::Z => Tetromino {
                geometry: Geometry {
                    shape: vec![(1, 1), (2, 1), (2, 0), (3, 0)],
                    center: (2, 0),
                    direction: CardinalDirection::North,
                },
                color: if *config::USE_XTERM_256_COLORS { Color::AnsiValue(196) } else { Color::Red },
                variant,
            },
        }
    }

    pub fn shift(&mut self, direction: ShiftDirection, board: &Board) -> bool {
        let (dx, can_shift) = match direction {
            ShiftDirection::Left => (-1, !board.hitting_left(self)),
            ShiftDirection::Right => (1, !board.hitting_right(self)),
        };

        if can_shift {
            self.geometry.transform(dx, 0);
        }

        can_shift
    }

    pub fn rotate(&mut self, direction: RotationDirection, board: &Board) -> bool {
        let mut can_rotate = false;

        let mut rotated = self.clone();
        rotated.geometry.rotate(direction);

        let offset_table = match self.variant {
            TetrominoVariant::J |
            TetrominoVariant::L |
            TetrominoVariant::S |
            TetrominoVariant::T |
            TetrominoVariant::Z => JLSTZ_OFFSETS,
            TetrominoVariant::I => I_OFFSETS,
            TetrominoVariant::O => O_OFFSETS,
        };

        for i in 0..offset_table[0].len() {
            let offset_x = offset_table[rotated.geometry.direction as usize][i].0
                - offset_table[self.geometry.direction as usize][i].0;
            let offset_y = offset_table[rotated.geometry.direction as usize][i].1
                - offset_table[self.geometry.direction as usize][i].1;

            rotated.geometry.transform(-offset_x, -offset_y);

            can_rotate = !board.overlapping(&rotated);

            if can_rotate {
                *self = rotated;
                break
            }

            rotated.geometry.transform(offset_x, offset_y);
        }

        can_rotate
    }

    pub fn start_pos_transform(&mut self, board: &Board) {
        self.geometry.transform(3, 18);
        for i in 17..20 {
            if board[i].iter().any(|block| block.is_some()) {
                self.geometry.transform(0, 1);
            }
        }
    }

    pub fn at_pos(&self, x: u16, y: u16, x_offset: u16, y_offset: u16) -> bool {
        self.geometry.shape.iter().any(|d| {
            let t_x = x_offset + (d.0 as u16 + 1) * 2;
            let t_y = y_offset + (d.1 as u16);
            t_y == y && (t_x == x || t_x - 1 == x)
        })
    }
}

