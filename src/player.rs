use std::{collections::HashSet, io::Result, mem::replace, ops::{Index, IndexMut}, pin::Pin, time::Duration};
use crossterm::style::Color;
use rand::rngs::StdRng;
use tokio::time::{sleep, Sleep};

use crate::{conn::ConnTrait, display::BOARD_DIMENSION, game::{rand_bag_gen, RotationDirection, ShiftDirection}, tetromino::*};

pub type Stack = Vec<Vec<Option<Color>>>;

const LOCK_RESET_LIMIT: u8 = 15;
const LOCK_DURATION: Duration = Duration::from_millis(500);
const LINE_CLEAR_DURATION: Duration = Duration::from_millis(125);

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

#[derive(Copy, Clone)]
pub enum PlayerKind {
    Local,
    Remote,
}

impl<T> Index<PlayerKind> for Vec<T> {
    type Output = T;
    fn index(&self, rg: PlayerKind) -> &T {
        &self[rg as usize]
    }
}

impl<T> IndexMut<PlayerKind> for Vec<T> {
    fn index_mut(&mut self, rg: PlayerKind) -> &mut T {
        &mut self[rg as usize]
    }
}

pub struct Player {
    pub kind: PlayerKind,
    pub falling: Tetromino,
    pub holding: Option<Tetromino>,
    pub ghost: Option<Tetromino>,
    pub next: Vec<Tetromino>,
    pub bag: Vec<Tetromino>,
    pub seed: StdRng,
    pub stack: Stack,
    pub start_level: u32,
    pub level: u32,
    pub lines: u32,
    pub score: u32,
    pub combo: i32,
    pub clearing: HashSet<usize>,
    pub can_hold: bool,
    pub lost: bool,
    pub locking: bool,
    pub lock_reset_count: u8,
}

impl Player {
    pub fn new(kind: PlayerKind, start_level: u32, seed: &mut StdRng) -> Self {
        let mut bag = rand_bag_gen(seed);
        Player {
            kind,
            falling: bag.pop().unwrap(),
            holding: None,
            ghost: None,
            next: bag.split_off(bag.len() - 3),
            bag,
            seed: seed.clone(),
            stack: vec![vec![None; BOARD_DIMENSION.0 as usize]; BOARD_DIMENSION.1 as usize],
            start_level,
            level: start_level,
            lines: 0,
            score: 0,
            combo: -1,
            clearing: HashSet::new(),
            can_hold: true,
            lost: false,
            locking: false,
            lock_reset_count: 0,
        }
    }

    pub fn calc_score(&mut self, num_cleared: u32) {
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

    pub fn get_next(&mut self) -> Tetromino {
        self.next.push(self.bag.pop().unwrap());
        if self.bag.is_empty() {
            self.bag = rand_bag_gen(&mut self.seed);
        }
        self.next.remove(0)
    }

    pub fn update_ghost(&mut self) {
        let mut ghost = self.falling.clone();
        while !ghost.hitting_bottom(&self.stack) {
            ghost.geometry.transform(0, -1);
        }
        self.ghost = if ghost.overlapping(&self.stack) {
            None
        } else {
            Some(ghost)
        };
    }

    pub fn mark_clear(&mut self) {
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

    fn reset_lock_timer(&mut self, lock_delay: &mut Pin<&mut Sleep>) {
        if self.lock_reset_count < LOCK_RESET_LIMIT {
            lock_delay.set(sleep(LOCK_DURATION));
        }
    }

    pub fn set_falling_geometry(&mut self, geometry: Geometry) {
        self.falling.geometry = geometry;
        self.update_ghost();
        self.locking = self.falling.hitting_bottom(&self.stack);
    }

    fn handle_shift(&mut self, direction: ShiftDirection) {
        match direction {
            ShiftDirection::Left => {
                if !self.falling.hitting_left(&self.stack) {
                    self.falling.geometry.transform(-1, 0);
                    self.update_ghost();
                }
            },
            ShiftDirection::Right => {
                if !self.falling.hitting_right(&self.stack) {
                    self.falling.geometry.transform(1, 0);
                    self.update_ghost();
                }
            },
        }
    }

    pub async fn shift(
        &mut self,
        direction: ShiftDirection,
        lock_delay: &mut Pin<&mut Sleep>,
        line_clear_delay: &mut Pin<&mut Sleep>,
        conn: &Box<dyn ConnTrait>,
    ) -> Result<()> {
        match self.kind {
            PlayerKind::Local => {
                if self.lock_reset_count == LOCK_RESET_LIMIT {
                    self.place(line_clear_delay, conn).await?;
                }
                self.handle_shift(direction);
                self.lock_reset_count += 1;
                self.reset_lock_timer(lock_delay);
                conn.send_pos(&self).await?;
            },
            PlayerKind::Remote => {
                self.handle_shift(direction);
            },
        }
        Ok(())
    }

    pub async fn rotate(
        &mut self,
        direction: RotationDirection,
        lock_delay: &mut Pin<&mut Sleep>,
        conn: &Box<dyn ConnTrait>,
    ) -> Result<()> {
        let mut rotated = self.falling.clone();
        rotated.geometry.rotate(direction);

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
            let offset_x = offset_table[rotated.geometry.direction as usize][i].0
                - offset_table[self.falling.geometry.direction as usize][i].0;
            let offset_y = offset_table[rotated.geometry.direction as usize][i].1
                - offset_table[self.falling.geometry.direction as usize][i].1;

            rotated.geometry.transform(-offset_x, -offset_y);

            if !rotated.overlapping(&self.stack) {
                self.falling = rotated;
                self.lock_reset_count += 1;
                self.update_ghost();
                self.reset_lock_timer(lock_delay);

                if let PlayerKind::Local = self.kind {
                    conn.send_pos(self).await?;
                }

                return Ok(())
            }

            rotated.geometry.transform(offset_x, offset_y);
        }

        Ok(())
    }

    pub async fn place(
        &mut self,
        line_clear_delay: &mut Pin<&mut Sleep>,
        conn: &Box<dyn ConnTrait>,
    ) -> Result<()> {
        if !self.falling.hitting_bottom(&self.stack) {
            return Ok(())
        }

        for position in self.falling.geometry.shape.iter() {
            if position.1 > BOARD_DIMENSION.1 - 1 {
                self.lost = true;
                return Ok(())
            }
            self.stack[position.1 as usize][position.0 as usize] = Some(self.falling.color);
        }

        self.mark_clear();

        if let PlayerKind::Local = self.kind {
            conn.send_place(self).await?;
        }

        let mut falling = self.get_next();
        for i in 17..20 {
            if self.stack[i].iter().any(|block| block.is_some()) {
                falling.geometry.transform(0, 1);
            }
        }

        self.falling = falling;
        self.locking = false;
        self.can_hold = true;

        self.update_ghost();

        line_clear_delay.set(sleep(LINE_CLEAR_DURATION));

        Ok(())
    }

    pub async fn hold(&mut self, conn: &Box<dyn ConnTrait>) -> Result<()> {
        if self.can_hold {
            let swap = self.holding.clone().unwrap_or_else(|| self.get_next());

            self.holding = Some(Tetromino::new(self.falling.variant));
            self.falling = swap;
            self.can_hold = false;

            self.update_ghost();

            if let PlayerKind::Local = self.kind {
                conn.send_hold().await?;
            }
        }
        Ok(())
    }

    pub fn drop(&mut self, lock_delay: &mut Pin<&mut Sleep>) {
        if !self.falling.hitting_bottom(&self.stack) {
            self.falling.geometry.transform(0, -1);
            self.lock_reset_count = 0;
            self.reset_lock_timer(lock_delay);
        }
        self.locking = self.falling.hitting_bottom(&self.stack);
    }

    pub async fn soft_drop(
        &mut self,
        lock_delay: &mut Pin<&mut Sleep>,
        conn: &Box<dyn ConnTrait>,
    ) -> Result<()> {
        self.drop(lock_delay);
        if !self.falling.hitting_bottom(&self.stack) {
            self.score += 1;
        }
        conn.send_pos(self).await?;
        Ok(())
    }

    pub async fn hard_drop(
        &mut self,
        line_clear_delay: &mut Pin<&mut Sleep>,
        conn: &Box<dyn ConnTrait>,
    ) -> Result<()> {
        while !self.falling.hitting_bottom(&self.stack) {
            self.falling.geometry.transform(0, -1);
            self.score += 2;
        }
        self.place(line_clear_delay, conn).await?;
        Ok(())
    }
}

