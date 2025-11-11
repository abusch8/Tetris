use std::io::Result;

use crate::{conn::ConnTrait, player::{Player, ShiftDirection}, stack::Stack, tetromino::{RotationDirection, Tetromino}};

fn check_valid_play(tetromino: &Tetromino, stack: &Stack) -> bool {
    !tetromino.overlapping(stack) && tetromino.hitting_bottom(stack) // TODO reachability check
}

pub struct Agent {
    pub goal: Option<Tetromino>,
}

impl Agent {
    pub fn new() -> Self {
        Agent { goal: None }
    }

    fn scan(player: &Player) -> Vec<Tetromino> {
        let mut valid_plays = Vec::new();

        let mut tetromino = Tetromino::new(player.falling.variant);

        while tetromino.geometry.center.1 > 0 {
            tetromino.geometry.transform(0, -1);
        }
        while tetromino.geometry.center.0 > 0 {
            tetromino.geometry.transform(-1, 0);
        }

        for i in 0..player.stack.len() {
            for _ in 0..player.stack.len() {
                for _ in 0..4 {
                    tetromino.geometry.rotate(RotationDirection::Clockwise);
                    if check_valid_play(&tetromino, &player.stack) {
                        valid_plays.push(tetromino.clone());
                    }
                }
                tetromino.geometry.transform(if i % 2 == 0 { 1 } else { -1 }, 0);
            }
            tetromino.geometry.transform(0, 1);
        }

        valid_plays
    }

    pub fn evaluate(&mut self, player: &Player) {
        let valid_plays = Self::scan(player);

        let mut scores: Vec<(Tetromino, i32)> = Vec::new();

        for play in valid_plays {
            let mut stack = Stack(player.stack.clone());
            stack.add(&play);
            let mut score = 0;
            score += stack.evaluate_gaps();
            score += stack.evaluate_height();
            scores.push((play, score));

        }

        scores.sort_by(|a, b| b.1.cmp(&a.1));

        // debug_log!("{:?}", scores.iter().map(|s| (s.1, s.0.geometry.center)).collect::<Vec<(i32, (i32, i32))>>());

        self.goal = Some(scores[0].0.clone());
    }

    pub async fn execute(&mut self, player: &mut Player, conn: &Box<dyn ConnTrait>) -> Result<()> {

        if let Some(goal) = &self.goal {
            if player.falling.geometry.direction != goal.geometry.direction {
                player.falling.geometry.rotate(RotationDirection::Clockwise);
            } else if player.falling.geometry.center.0 > goal.geometry.center.0 {
                player.shift(ShiftDirection::Left, conn).await?;
            } else if player.falling.geometry.center.0 < goal.geometry.center.0 {
                player.shift(ShiftDirection::Right, conn).await?;
            } else {
                player.hard_drop(conn).await?;
                self.evaluate(player);
            }
        }

        Ok(())
    }
}

