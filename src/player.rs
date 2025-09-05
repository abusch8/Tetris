use std::{collections::HashSet, io::Result, mem::replace, ops::{Deref, DerefMut}, pin::Pin, time::Duration};
use crossterm::style::Color;
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use strum::IntoEnumIterator;
use tokio::time::{interval, sleep, Interval, Sleep};
use num_derive::FromPrimitive;

use crate::{agent, conn::ConnTrait, display::BOARD_DIMENSION, score::{ClearKind, Score}, tetromino::*};

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


#[derive(FromPrimitive, PartialEq)]
pub enum ShiftDirection { Left, Right }

#[derive(Copy, Clone)]
pub enum PlayerKind { Ai, Local, Remote }

pub struct Stack(pub Vec<Vec<Option<Color>>>);

impl Stack {
    pub fn new() -> Self {
        Stack(vec![vec![None; BOARD_DIMENSION.0 as usize]; BOARD_DIMENSION.1 as usize])
    }

    pub fn line_clear(&mut self, clearing: &HashSet<usize>) { // TODO refactor
        let stack = replace(self, Stack(Vec::new()));

        for (i, row) in stack.clone().into_iter().enumerate() {
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

    pub fn evaluate_roughness(&self) -> i32 {

        0
    }

    pub fn evaluate_height(&self) -> i32 {
        self.iter()
            .enumerate()
            .find(|(_, row)| row.iter().any(|k| k.is_some()))
            .map(|(i, _)| i)
            .unwrap_or(self.len()) as i32
    }
}

impl Deref for Stack {
    type Target = Vec<Vec<Option<Color>>>;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Stack {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

pub struct Bag {
    pub next: Vec<Tetromino>,
    pub rest: Vec<Tetromino>,
}

impl Bag {
    fn new(seed: &mut StdRng) -> Self {
        let mut bag = Bag::rand_bag_gen(seed);
        Bag {
            next: bag.split_off(bag.len() - 3),
            rest: bag,
        }
    }

    fn rand_bag_gen(seed: &mut StdRng) -> Vec<Tetromino> {
        let mut bag = TetrominoVariant::iter()
            .map(|variant| Tetromino::new(variant))
            .collect::<Vec<Tetromino>>();

        bag.shuffle(seed);
        bag
    }

    fn get_next(&mut self, seed: &mut StdRng) -> Tetromino {
        self.next.push(self.rest.pop().unwrap());
        if self.rest.is_empty() {
            self.rest = Bag::rand_bag_gen(seed);
        }
        self.next.remove(0)
    }
}

pub struct Player {
    pub kind: PlayerKind,
    pub seed: StdRng,
    pub falling: Tetromino,
    pub holding: Option<Tetromino>,
    pub ghost: Option<Tetromino>,
    pub bag: Bag,
    pub stack: Stack,
    pub score: Score,
    pub clearing: HashSet<usize>,
    pub can_hold: bool,
    pub lost: bool,
    pub locking: bool,
    pub lock_reset_count: u8,
    pub last_action_was_rotate: bool,
    pub drop_interval: Interval,
}

impl Player {
    pub fn new(kind: PlayerKind, start_level: u32, seed: u64) -> Self {
        let mut seed = StdRng::seed_from_u64(seed);
        let mut bag = Bag::new(&mut seed);
        let stack = Stack::new();
        let mut falling = bag.get_next(&mut seed);
        falling.start_pos_transform(&stack);
        Player {
            kind,
            seed,
            falling,
            holding: None,
            ghost: None,
            bag,
            stack,
            score: Score::new(start_level),
            clearing: HashSet::new(),
            can_hold: true,
            lost: false,
            locking: false,
            lock_reset_count: 0,
            last_action_was_rotate: false,
            drop_interval: Player::calc_drop_interval(start_level),
        }
    }

    fn calc_drop_interval(level: u32) -> Interval {
        let drop_rate = (0.8 - (level - 1) as f32 * 0.007).powf((level - 1) as f32);
        let drop_duration = Duration::from_nanos((drop_rate * 1_000_000_000f32) as u64);

        interval(if drop_duration.is_zero() {
            Duration::from_nanos(1)
        } else {
            drop_duration
        })
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

    pub fn mark_clear(&mut self, line_clear_delay: &mut Pin<&mut Sleep>) {
        self.clearing = HashSet::new();
        for (i, row) in self.stack.iter().enumerate() {
            if row.iter().all(|cell| cell.is_some()) {
                self.clearing.insert(i);
            }
        }
        if self.clearing.is_empty() {
            self.score.combo = -1;
        } else {
            line_clear_delay.set(sleep(LINE_CLEAR_DURATION));
        }
    }

    pub fn t_spin_check(&self) -> bool {
        if !self.last_action_was_rotate || !matches!(self.falling.variant, TetrominoVariant::T) {
            return false;
        }
        let (c_x, c_y) = self.falling.geometry.center;

        let corners = vec![(-1, -1), (-1, 1), (1, -1), (1, 1)];
        let corners_occupied = corners
            .iter()
            .filter(|&&(d_x, d_y)| {
                let x = c_x + d_x;
                let y = c_y + d_y;
                self.stack
                    .get(x as usize)
                    .and_then(|col| col.get(y as usize))
                    .map_or(true, |cell| cell.is_some())
            })
            .count();

        corners_occupied >= 3
    }

    pub fn line_clear(&mut self) -> ClearKind {
        self.stack.line_clear(&self.clearing);

        let clear_kind = ClearKind::from_state(&self);

        self.score.score_clear(clear_kind);
        self.update_ghost();
        self.drop_interval = Player::calc_drop_interval(self.score.level);

        self.clearing.clear();

        clear_kind
    }

    pub fn add_garbage(&mut self, clear_kind: ClearKind) {
        self.stack.add_garbage(clear_kind, &mut self.seed);
        while self.falling.overlapping(&self.stack) {
            self.falling.geometry.transform(0, -1);
        }
        self.update_ghost();
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
        self.last_action_was_rotate = false;
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
            PlayerKind::Remote | PlayerKind::Ai=> {
                self.handle_shift(direction);
            },
        }
        self.last_action_was_rotate = false;
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

                self.last_action_was_rotate = true;

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

        if !self.stack.add(&self.falling) {
            self.lost = true;
            return Ok(())
        }

        self.mark_clear(line_clear_delay);

        if let PlayerKind::Local = self.kind {
            conn.send_place(self).await?;
        }

        let mut falling = self.bag.get_next(&mut self.seed);
        falling.start_pos_transform(&self.stack);

        self.falling = falling;
        self.locking = false;
        self.can_hold = true;

        self.update_ghost();

        self.last_action_was_rotate = false;

        Ok(())
    }

    pub async fn hold(&mut self, conn: &Box<dyn ConnTrait>) -> Result<()> {
        if self.can_hold {
            let mut swap = self.holding.clone().unwrap_or(self.bag.get_next(&mut self.seed));
            swap.start_pos_transform(&self.stack);

            let tetromino = Tetromino::new(self.falling.variant);
            self.holding = Some(tetromino);
            self.falling = swap;
            self.can_hold = false;

            self.update_ghost();

            if let PlayerKind::Local = self.kind {
                conn.send_hold().await?;
            }

            self.last_action_was_rotate = false;
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
            self.score.score += 1;
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
            self.score.score += 2;
        }
        self.place(line_clear_delay, conn).await?;
        Ok(())
    }
}

