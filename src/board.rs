use std::{collections::HashSet, mem::replace, ops::{Deref, DerefMut}};
use crossterm::style::Color;
use rand::{rngs::StdRng, Rng};

use crate::{score::ClearKind, tetromino::*};

pub type Dimension = (i32, i32);

pub const BOARD_DIMENSION: Dimension = (10, 20);
pub const BOARD_MP_OFFSET: u16 = 30;

pub struct Board(pub Vec<Vec<Option<Color>>>);

impl Board {
    pub fn new() -> Self {
        Board(vec![vec![None; BOARD_DIMENSION.0 as usize]; BOARD_DIMENSION.1 as usize])
    }

    pub fn line_clear(&mut self, clearing: &HashSet<usize>) { // TODO refactor
        let board = replace(self, Board(Vec::new()));

        for (i, row) in board.clone().into_iter().enumerate() {
            if clearing.get(&i).is_none() {
                self.push(row);
            }
        }

        self.extend(vec![vec![None; BOARD_DIMENSION.0 as usize]; clearing.len()]);
    }

    pub fn add(&mut self, tetromino: &Tetromino) -> bool {
        for position in tetromino.geometry.shape.iter() {
            if position.1 > BOARD_DIMENSION.1 - 1 {
                return false;
            }
            self[position.1 as usize][position.0 as usize] = Some(tetromino.color);
        }
        true
    }

    pub fn add_garbage(&mut self, clear_kind: ClearKind, seed: &mut StdRng) {
        let hole = seed.gen_range(0..10);
        let line = (0..10).map(|i| if i == hole { None } else { Some(Color::White) }).collect();
        let garbage = vec![line; clear_kind.garbage_line_count()];
        self.splice(0..0, garbage);
    }

    pub fn evaluate_gaps(&self) -> i32 {
        let mut score = 0;

        for row in self.iter() {
            let mut x = 0;
            for k in row.iter() {
                if k.is_some() {
                    x += 1;
                }
                score += x / 10
            }
        }
        score
    }

    pub fn evaluate_height(&self) -> i32 {
        self.iter()
            .enumerate()
            .find(|(_, row)| row.iter().any(|k| k.is_some()))
            .map(|(i, _)| i)
            .unwrap_or(self.len()) as i32
    }

    pub fn overlapping(&self, tetromino: &Tetromino) -> bool {
        tetromino.geometry.shape.iter().any(|position| {
            position.0 < 0 ||
            position.1 < 0 ||
            position.0 > BOARD_DIMENSION.0 - 1 ||
            position.1 > BOARD_DIMENSION.1 - 1 ||
            self[position.1 as usize][position.0 as usize].is_some()
        })
    }

    pub fn hitting_bottom(&self, tetromino: &Tetromino) -> bool {
        tetromino.geometry.shape.iter().any(|position| {
            position.1 == 0 ||
            position.1 < BOARD_DIMENSION.1 &&
            self[(position.1 - 1) as usize][position.0 as usize].is_some()
        })
    }

    pub fn hitting_left(&self, tetromino: &Tetromino) -> bool {
        tetromino.geometry.shape.iter().any(|position| {
            position.0 == 0 ||
            position.1 < BOARD_DIMENSION.1 &&
            self[position.1 as usize][(position.0 - 1) as usize].is_some()
        })
    }

    pub fn hitting_right(&self, tetromino: &Tetromino) -> bool {
        tetromino.geometry.shape.iter().any(|position| {
            position.0 == BOARD_DIMENSION.0 - 1 ||
            position.1 < BOARD_DIMENSION.1 &&
            self[position.1 as usize][(position.0 + 1) as usize].is_some()
        })
    }
}

impl Deref for Board {
    type Target = Vec<Vec<Option<Color>>>;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Board {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

