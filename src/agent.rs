use std::{io::Result, pin::Pin, thread::sleep, time::Duration};

use crate::{conn::ConnTrait, debug_log, display::{Dimension, BOARD_DIMENSION}, player::{Player, ShiftDirection, Stack}, tetromino::{CardinalDirection, RotationDirection, Tetromino}};
use tokio::time::Sleep;

use strum::IntoEnumIterator;

fn check_valid_play(tetromino: &Tetromino, stack: &Stack) -> bool {
    !tetromino.overlapping(stack) && tetromino.hitting_bottom(stack) // TODO reachability check
}

fn scan(player: &Player) -> Vec<Tetromino> {
    let mut valid_plays = Vec::new();

    let mut t = Tetromino::new(player.falling.variant);

    while t.geometry.center.1 > 0 {
        t.geometry.transform(0, -1);
    }
    while t.geometry.center.0 > 0 {
        t.geometry.transform(-1, 0);
    }

    for i in 0..player.stack.len() {
        for _ in 0..player.stack.len() {
            for _ in 0..4 {
                t.geometry.rotate(RotationDirection::Clockwise);
                if check_valid_play(&t, &player.stack) {
                    valid_plays.push(t.clone());
                }
            }
            t.geometry.transform(if i % 2 == 0 { 1 } else { -1 }, 0);
        }
        t.geometry.transform(0, 1);
    }

    valid_plays
}

fn evaluate(player: &Player, valid_plays: Vec<Tetromino>) -> Tetromino {
    let mut scores: Vec<(Tetromino, i32)> = Vec::new();

    for play in valid_plays {
        let mut stack = Stack(player.stack.clone());
        stack.add(&play);
        let mut score = 0;
        score += stack.evaluate_gaps();
        score += stack.evaluate_roughness();
        score += stack.evaluate_height();
        scores.push((play, score));

    }

    scores.sort_by(|a, b| b.1.cmp(&a.1));

    // debug_log!("{:?}", scores.iter().map(|s| (s.1, s.0.geometry.center)).collect::<Vec<(i32, (i32, i32))>>());

    scores[0].0.clone()

}

pub async fn execute(player: &mut Player, lock_delay: &mut Pin<&mut Sleep>, line_clear_delay: &mut Pin<&mut Sleep>, conn: &Box<dyn ConnTrait>) -> Result<()> {
    let valid_plays = scan(player);
    let best_play = evaluate(player, valid_plays);

    while player.falling.geometry.direction != best_play.geometry.direction {
        player.falling.geometry.rotate(RotationDirection::Clockwise);
    }
    while player.falling.geometry.center.0 > best_play.geometry.center.0 {
        player.falling.geometry.transform(-1, 0);
    }
    while player.falling.geometry.center.0 < best_play.geometry.center.0 {
        player.falling.geometry.transform(1, 0);
    }

    Box::pin(player.hard_drop(lock_delay, line_clear_delay, conn)).await?;

    Ok(())
}

