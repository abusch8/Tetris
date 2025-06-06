use std::io::Result;
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};

use crate::{conn::{Conn, ConnTrait}, debug_println, player::Player};

pub struct Game {
    pub player: Vec<Player>,
}

impl Game {
    pub async fn start(start_level: u32, conn: &mut Box<dyn ConnTrait>) -> Result<Self> {
        let seed_idx = conn.is_host() as usize;

        let mut seeds = Game::generate_seeds(conn).await?;

        let mut game = Game {
            player: vec![
                Player::new(start_level, &mut seeds[seed_idx ^ 1]),
            ],
        };
        if conn.is_multiplayer() {
            game.player.push(
                Player::new(start_level, &mut seeds[seed_idx]),
            );
        }
        game.player
            .iter_mut()
            .for_each(|p| p.update_ghost());

        Ok(game)
    }

    async fn generate_seeds(conn: &mut Box<dyn ConnTrait>) -> Result<Vec<StdRng>> {
        if conn.is_multiplayer() {
            if conn.is_host() {
                let p1_seed = thread_rng().gen::<u64>();
                let p2_seed = thread_rng().gen::<u64>();
                conn.send_seeds(p1_seed, p2_seed).await?;
                Ok(vec![
                    StdRng::seed_from_u64(p1_seed),
                    StdRng::seed_from_u64(p2_seed),
                ])
            } else {
                let (p1_seed, p2_seed) = conn.recv_seeds().await?;
                Ok(vec![
                    StdRng::seed_from_u64(p1_seed),
                    StdRng::seed_from_u64(p2_seed),
                ])
            }
        } else {
            Ok(vec![StdRng::seed_from_u64(thread_rng().gen::<u64>())])
        }
    }
}

