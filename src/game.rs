use std::io::Result;
use rand::{rngs::StdRng, seq::SliceRandom, thread_rng, Rng, SeedableRng};
use num_derive::FromPrimitive;
use strum::IntoEnumIterator;

use crate::{conn::{ConnKind, ConnTrait, TcpPacketMode}, debug_log, player::{Player, PlayerKind}, tetromino::{Tetromino, TetrominoVariant}};

#[derive(FromPrimitive, PartialEq)]
pub enum ShiftDirection { Left, Right }

#[derive(PartialEq)]
pub enum RotationDirection { Clockwise, CounterClockwise }

pub fn rand_bag_gen(seed: &mut StdRng) -> Vec<Tetromino> {
    let mut bag = TetrominoVariant::iter()
        .map(|variant| Tetromino::new(variant))
        .collect::<Vec<Tetromino>>();

    bag.shuffle(seed);
    bag
}

pub struct Game {
    pub players: Vec<Player>,
}

impl Game {
    pub async fn start(conn_kind: ConnKind, start_level: u32, conn: &mut Box<dyn ConnTrait>) -> Result<Self> {
        let seed_idx = conn_kind.is_host() as usize;

        let mut seeds = Game::generate_seeds(conn_kind, conn).await?;

        let mut game = Game {
            players: vec![
                Player::new(PlayerKind::Local, start_level, &mut seeds[seed_idx ^ 1]),
            ],
        };
        if conn_kind.is_multiplayer() {
            game.players.push(
                Player::new(PlayerKind::Remote, start_level, &mut seeds[seed_idx]),
            );
        }
        game.players
            .iter_mut()
            .for_each(|p| p.update_ghost());

        Ok(game)
    }

    async fn generate_seeds(conn_kind: ConnKind, conn: &Box<dyn ConnTrait>) -> Result<Vec<StdRng>> {
        match conn_kind {
            ConnKind::Host => {
                let p1_seed = thread_rng().gen::<u64>();
                let p2_seed = thread_rng().gen::<u64>();
                conn.send_seeds(p1_seed, p2_seed).await?;
                Ok(vec![
                    StdRng::seed_from_u64(p1_seed),
                    StdRng::seed_from_u64(p2_seed),
                ])
            },
            ConnKind::Client => {
                loop {
                    let (mode, payload) = conn.recv_tcp().await?;
                    match mode {
                        TcpPacketMode::Seeds => {
                            let p1_seed_bytes: [u8; 8] = payload[0..8].try_into().unwrap();
                            let p2_seed_bytes: [u8; 8] = payload[8..16].try_into().unwrap();
                            let p1_seed = u64::from_le_bytes(p1_seed_bytes);
                            let p2_seed = u64::from_le_bytes(p2_seed_bytes);
                            return Ok(vec![
                                StdRng::seed_from_u64(p1_seed),
                                StdRng::seed_from_u64(p2_seed),
                            ]);
                        },
                        _ => (),
                    }
                }
            },
            ConnKind::Empty => {
                Ok(vec![
                    StdRng::seed_from_u64(thread_rng().gen::<u64>()),
                ])
            },
        }
    }
}

