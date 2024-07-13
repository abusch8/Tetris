use std::{collections::HashSet, mem::replace, pin::Pin};
use crossterm::style::Color;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use rand::{thread_rng, seq::SliceRandom};
use strum::IntoEnumIterator;
use tokio::time::{sleep, Sleep};

use crate::{display::BOARD_DIMENSION, run::{LINE_CLEAR_DURATION, LOCK_DURATION, LOCK_RESET_LIMIT}, tetromino::*};

#[derive(FromPrimitive, PartialEq)]
pub enum ShiftDirection { Left, Right, Down }

#[derive(PartialEq)]
pub enum RotationDirection { Clockwise, CounterClockwise }

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

fn rand_bag_gen() -> Vec<Tetromino> {
    let mut bag = TetrominoVariant::iter()
        .map(|variant| Tetromino::new(variant))
        .collect::<Vec<Tetromino>>();

    bag.shuffle(&mut thread_rng());
    bag
}

pub struct Game {
    pub falling: Tetromino,
    pub holding: Option<Tetromino>,
    pub ghost: Option<Tetromino>,
    pub next: Vec<Tetromino>,
    pub bag: Vec<Tetromino>,
    pub stack: Vec<Vec<Option<Color>>>,
    pub start_level: u32,
    pub score: u32,
    pub level: u32,
    pub lines: u32,
    pub combo: i32,
    pub clearing: HashSet<usize>,
    pub can_hold: bool,
    pub locking: bool,
    pub lock_reset_count: u8,
    pub end: bool,
}

impl Game {
    pub fn start(start_level: u32) -> Self {
        let mut bag = rand_bag_gen();
        let mut game = Game {
            falling: bag.pop().unwrap(),
            holding: None,
            ghost: None,
            next: bag.split_off(bag.len() - 3),
            bag,
            stack: vec![vec![None; BOARD_DIMENSION.0 as usize]; BOARD_DIMENSION.1 as usize],
            start_level,
            score: 0,
            level: start_level,
            lines: 0,
            combo: -1,
            clearing: HashSet::new(),
            can_hold: true,
            locking: false,
            lock_reset_count: 0,
            end: false,
        };
        game.update_ghost();
        game
    }

    fn get_next(&mut self) -> Tetromino {
        self.next.push(self.bag.pop().unwrap());
        if self.bag.is_empty() {
            self.bag = rand_bag_gen()
        }
        self.next.remove(0)
    }

    pub fn hitting_bottom(&self, tetromino: &Tetromino) -> bool {
        tetromino.shape.iter().any(|position| {
            position.1 == 0 ||
            position.1 < BOARD_DIMENSION.1 &&
            self.stack[(position.1 - 1) as usize][position.0 as usize].is_some()
        })
    }

    pub fn hitting_left(&self, tetromino: &Tetromino) -> bool {
        tetromino.shape.iter().any(|position| {
            position.0 == 0 ||
            position.1 < BOARD_DIMENSION.1 &&
            self.stack[position.1 as usize][(position.0 - 1) as usize].is_some()
        })
    }

    pub fn hitting_right(&self, tetromino: &Tetromino) -> bool {
        tetromino.shape.iter().any(|position| {
            position.0 == BOARD_DIMENSION.0 - 1 ||
            position.1 < BOARD_DIMENSION.1 &&
            self.stack[position.1 as usize][(position.0 + 1) as usize].is_some()
        })
    }

    fn update_ghost(&mut self) {
        let mut ghost = self.falling.clone();
        while !self.hitting_bottom(&ghost) {
            for position in ghost.shape.iter_mut() {
                position.1 -= 1;
            }
        }
        self.ghost = if self.overlapping(&ghost.shape) { None } else { Some(ghost) };
    }

    fn reset_lock_timer(&mut self, lock_delay: &mut Pin<&mut Sleep>) {
        if self.lock_reset_count < LOCK_RESET_LIMIT {
            lock_delay.set(sleep(LOCK_DURATION));
        }
    }

    pub fn shift(
        &mut self,
        direction: ShiftDirection,
        lock_delay: &mut Pin<&mut Sleep>,
        line_clear_delay: &mut Pin<&mut Sleep>,
    ) {
        if self.lock_reset_count == LOCK_RESET_LIMIT {
            self.place(line_clear_delay)
        }

        match direction {
            ShiftDirection::Left => {
                if !self.hitting_left(&self.falling) {
                    for position in self.falling.shape.iter_mut() {
                        position.0 -= 1;
                    }
                    self.falling.center.0 -= 1;
                    self.lock_reset_count += 1;
                    self.reset_lock_timer(lock_delay);
                }
            },
            ShiftDirection::Right => {
                if !self.hitting_right(&self.falling) {
                    for position in self.falling.shape.iter_mut() {
                        position.0 += 1;
                    }
                    self.falling.center.0 += 1;
                    self.lock_reset_count += 1;
                    self.reset_lock_timer(lock_delay);
                }
            },
            ShiftDirection::Down => {
                if !self.hitting_bottom(&self.falling) {
                    for position in self.falling.shape.iter_mut() {
                        position.1 -= 1;
                    }
                    self.falling.center.1 -= 1;
                    self.lock_reset_count = 0;
                    self.reset_lock_timer(lock_delay);
                }
                self.locking = self.hitting_bottom(&self.falling);
            },
        }

        self.update_ghost();
    }

    fn overlapping(&self, shape: &Shape) -> bool {
        shape.iter().any(|position| {
            position.0 < 0 ||
            position.1 < 0 ||
            position.0 > BOARD_DIMENSION.0 - 1 ||
            position.1 > BOARD_DIMENSION.1 - 1 ||
            self.stack[position.1 as usize][position.0 as usize].is_some()
        })
    }

    pub fn rotate(&mut self, direction: RotationDirection, lock_delay: &mut Pin<&mut Sleep>) {
        let (angle, new_direction) = match direction {
            RotationDirection::Clockwise => (
                f32::from(-90.0).to_radians(),
                CardinalDirection::from_i32((self.falling.direction as i32 + 1) % 4).unwrap(),
            ),
            RotationDirection::CounterClockwise => (
                f32::from(90.0).to_radians(),
                CardinalDirection::from_i32(((self.falling.direction as i32 - 1) % 4 + 4) % 4).unwrap(),
            ),
        };

        let rotated: Vec<(i32, i32)> = self.falling.shape.iter().map(|&(x, y)| {
            let x = (x - self.falling.center.0) as f32;
            let y = (y - self.falling.center.1) as f32;
            (
                ((x * angle.cos() - y * angle.sin()) + self.falling.center.0 as f32).round() as i32,
                ((x * angle.sin() + y * angle.cos()) + self.falling.center.1 as f32).round() as i32,
            )
        }).collect();

        let offset_table = match self.falling.variant {
            TetrominoVariant::J |
            TetrominoVariant::L |
            TetrominoVariant::S |
            TetrominoVariant::T |
            TetrominoVariant::Z => JLSTZ_OFFSETS,
            TetrominoVariant::I => I_OFFSETS,
            TetrominoVariant::O => O_OFFSETS,
        };

        for i in 0..offset_table[0].len() {
            let offset_x = offset_table[new_direction as usize][i].0
                - offset_table[self.falling.direction as usize][i].0;
            let offset_y = offset_table[new_direction as usize][i].1
                - offset_table[self.falling.direction as usize][i].1;

            let kicked = rotated.iter().map(|&(x, y)| (x - offset_x, y - offset_y)).collect();

            if !self.overlapping(&kicked) {
                self.falling.shape = kicked;
                self.falling.center.0 -= offset_x;
                self.falling.center.1 -= offset_y;
                self.falling.direction = new_direction;
                self.lock_reset_count += 1;
                self.update_ghost();
                self.reset_lock_timer(lock_delay);
                return
            }
        }
    }

    fn mark_clear(&mut self) {
        let mut clearing = HashSet::new();
        for (i, row) in self.stack.iter().enumerate() {
            if row.iter().all(|block| block.is_some()) {
                clearing.insert(i);
            }
        }
        self.clearing = clearing;
    }

    pub fn line_clear(&mut self) {
        let stack = replace(&mut self.stack, Vec::new());

        for (i, row) in stack.into_iter().enumerate() {
            if self.clearing.get(&i).is_none() {
                self.stack.push(row);
            }
        }

        let num_cleared = self.clearing.len() as u32;

        self.stack.extend(vec![vec![None; BOARD_DIMENSION.0 as usize]; num_cleared as usize]);

        if num_cleared > 0 {
            self.lines += num_cleared;
            self.level = self.start_level + self.lines / 10;
            self.combo += 1;
            self.calc_score(num_cleared);
            self.update_ghost();
        } else {
            self.combo = -1;
        }

        self.clearing.clear();
    }

    fn calc_score(&mut self, num_cleared: u32) {
        let full_clear = self.stack.iter().flatten().all(|block| block.is_none());
        self.score += if full_clear {
            match num_cleared {
                1 => self.level * 800,
                2 => self.level * 1200,
                3 => self.level * 1800,
                4 => self.level * 2000,
                _ => 0,
            }
        } else {
            match num_cleared {
                1 => self.level * 100,
                2 => self.level * 300,
                3 => self.level * 500,
                4 => self.level * 800,
                _ => 0,
            }
        };
        self.score += 50 * self.combo as u32 * self.level;
    }

    pub fn place(&mut self, line_clear_delay: &mut Pin<&mut Sleep>) {
        if !self.hitting_bottom(&self.falling) {
            return
        }

        for position in self.falling.shape.iter() {
            if position.1 > BOARD_DIMENSION.1 - 1 {
                self.end = true;
                return
            }
            self.stack[position.1 as usize][position.0 as usize] = Some(self.falling.color);
        }

        self.mark_clear();

        let mut falling = self.get_next();
        for i in 17..20 {
            if self.stack[i].iter().any(|block| block.is_some()) {
                for position in falling.shape.iter_mut() {
                    position.1 += 1;
                }
                falling.center.1 += 1;
            }
        }

        self.falling = falling;
        self.locking = false;
        self.can_hold = true;

        self.update_ghost();

        line_clear_delay.set(sleep(LINE_CLEAR_DURATION));
    }

    pub fn soft_drop(&mut self, lock_delay: &mut Pin<&mut Sleep>, line_clear_delay: &mut Pin<&mut Sleep>) {
        self.shift(ShiftDirection::Down, lock_delay, line_clear_delay);
        if !self.hitting_bottom(&self.falling) {
            self.score += 1;
        }
    }

    pub fn hard_drop(&mut self, line_clear_delay: &mut Pin<&mut Sleep>) {
        while !self.hitting_bottom(&self.falling) {
            for position in self.falling.shape.iter_mut() {
                position.1 -= 1;
                self.score += 2;
            }
        }
        self.place(line_clear_delay);
    }

    pub fn hold(&mut self) {
        if self.can_hold {
            let swap = self.holding.clone().unwrap_or_else(|| self.get_next());

            self.holding = Some(Tetromino::new(self.falling.variant));
            self.falling = swap;
            self.can_hold = false;

            self.update_ghost();
        }
    }
}

