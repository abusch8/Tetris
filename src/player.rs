use std::{collections::HashSet, io::Result, pin::Pin, time::Duration};
use rand::{rngs::StdRng, SeedableRng};
use tokio::time::{interval, sleep, Interval, Sleep};

use crate::{bag::Bag, board::Board, conn::ConnTrait, score::{ClearKind, Score}, tetromino::*};

const LOCK_RESET_LIMIT: u8 = 15;
const LOCK_DURATION: Duration = Duration::from_millis(500);
const LINE_CLEAR_DURATION: Duration = Duration::from_millis(125);

#[derive(Copy, Clone)]
pub enum PlayerKind { Computer, Local, Remote }

pub struct PlayerTimers {
    pub line_clear_delay: Pin<Box<Sleep>>,
    pub lock_delay: Pin<Box<Sleep>>,
}

pub struct Player {
    pub kind: PlayerKind,
    pub seed: StdRng,
    pub falling: Tetromino,
    pub holding: Option<Tetromino>,
    pub ghost: Option<Tetromino>,
    pub bag: Bag,
    pub board: Board,
    pub score: Score,
    pub clearing: HashSet<usize>,
    pub can_hold: bool,
    pub lost: bool,
    pub locking: bool,
    pub lock_reset_count: u8,
    pub last_action_was_rotate: bool,
    pub drop_interval: Interval,
    pub timers: PlayerTimers,
}

impl Player {
    pub fn new(kind: PlayerKind, start_level: u32, seed: u64) -> Self {
        let mut seed = StdRng::seed_from_u64(seed);
        let mut bag = Bag::new(&mut seed);

        let board = Board::new();

        let mut falling = bag.get_next(&mut seed);
        falling.start_pos_transform(&board);

        let timers = PlayerTimers {
            line_clear_delay: Box::pin(sleep(Duration::ZERO)),
            lock_delay: Box::pin(sleep(Duration::ZERO)),
        };

        Player {
            kind,
            seed,
            falling,
            holding: None,
            ghost: None,
            bag,
            board,
            score: Score::new(start_level),
            clearing: HashSet::new(),
            can_hold: true,
            lost: false,
            locking: false,
            lock_reset_count: 0,
            last_action_was_rotate: false,
            drop_interval: Player::calc_drop_interval(start_level),
            timers,
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
        while !self.board.hitting_bottom(&ghost) {
            ghost.geometry.transform(0, -1);
        }
        self.ghost = if self.board.overlapping(&ghost) {
            None
        } else {
            Some(ghost)
        };
    }

    pub fn mark_clear(&mut self) {
        self.clearing = HashSet::new();
        for (i, row) in self.board.iter().enumerate() {
            if row.iter().all(|cell| cell.is_some()) {
                self.clearing.insert(i);
            }
        }
        if self.clearing.is_empty() {
            self.score.combo = -1;
        } else {
            self.timers.line_clear_delay.set(sleep(LINE_CLEAR_DURATION));
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
                self.board
                    .get(x as usize)
                    .and_then(|col| col.get(y as usize))
                    .map_or(true, |cell| cell.is_some())
            })
            .count();

        corners_occupied >= 3
    }

    pub fn line_clear(&mut self) -> ClearKind {
        self.board.line_clear(&self.clearing);

        let clear_kind = ClearKind::from_state(&self);

        self.score.score_clear(clear_kind);
        self.update_ghost();
        self.drop_interval = Player::calc_drop_interval(self.score.level);

        self.clearing.clear();

        clear_kind
    }

    pub fn add_garbage(&mut self, clear_kind: ClearKind) {
        self.board.add_garbage(clear_kind, &mut self.seed);
        while self.board.overlapping(&self.falling) {
            self.falling.geometry.transform(0, -1);
        }
        self.update_ghost();
    }

    fn reset_lock_timer(&mut self) {
        if self.lock_reset_count < LOCK_RESET_LIMIT {
            self.timers.lock_delay.set(sleep(LOCK_DURATION));
        }
    }

    pub fn set_falling_geometry(&mut self, geometry: Geometry) {
        self.falling.geometry = geometry;
        self.update_ghost();
        self.locking = self.board.hitting_bottom(&self.falling);
    }

    pub async fn shift(&mut self, direction: ShiftDirection, conn: &Box<dyn ConnTrait>) -> Result<()> {
        if self.lock_reset_count == LOCK_RESET_LIMIT {
            self.place(conn).await?;
            return Ok(())
        }

        if self.falling.shift(direction, &self.board) {
            self.lock_reset_count += 1;
            self.reset_lock_timer();
            self.update_ghost();

            if let PlayerKind::Local = self.kind {
                conn.send_pos(&self).await?;
            }

            self.last_action_was_rotate = false;
        }

        Ok(())
    }

    pub async fn rotate(&mut self, direction: RotationDirection, conn: &Box<dyn ConnTrait>) -> Result<()> {
        if self.falling.rotate(direction, &self.board) {
            self.lock_reset_count += 1;
            self.reset_lock_timer();
            self.update_ghost();

            if let PlayerKind::Local = self.kind {
                conn.send_pos(self).await?;
            }

            self.last_action_was_rotate = true;
        }

        Ok(())
    }

    pub async fn place(&mut self, conn: &Box<dyn ConnTrait>) -> Result<()> {
        if !self.board.hitting_bottom(&self.falling) {
            return Ok(())
        }

        if !self.board.add(&self.falling) {
            self.lost = true;
            return Ok(())
        }

        self.mark_clear();

        if let PlayerKind::Local = self.kind {
            conn.send_place(self).await?;
        }

        let mut falling = self.bag.get_next(&mut self.seed);
        falling.start_pos_transform(&self.board);

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
            swap.start_pos_transform(&self.board);

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

    pub fn drop(&mut self) {
        if !self.board.hitting_bottom(&self.falling) {
            self.falling.geometry.transform(0, -1);
            self.lock_reset_count = 0;
            self.reset_lock_timer();
        }
        self.locking = self.board.hitting_bottom(&self.falling);
    }

    pub async fn soft_drop(&mut self, conn: &Box<dyn ConnTrait>) -> Result<()> {
        self.drop();

        if !self.board.hitting_bottom(&self.falling) {
            self.score.score += 1;
        }

        conn.send_pos(self).await?;

        Ok(())
    }

    pub async fn hard_drop(&mut self, conn: &Box<dyn ConnTrait>) -> Result<()> {
        while !self.board.hitting_bottom(&self.falling) {
            self.falling.geometry.transform(0, -1);
            self.score.score += 2;
        }

        self.place(conn).await?;

        Ok(())
    }
}

