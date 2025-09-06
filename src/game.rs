use std::io::Result;
use rand::{thread_rng, Rng};

use crate::{conn::{ConnKind, ConnTrait, TcpPacketMode}, player::{Player, PlayerKind}, Mode};

pub struct Game {
    pub player: Player,
    pub opponent: Option<Player>,
}

pub struct GameInfo {
    pub start_level: u32,
    pub seeds: Vec<u64>,
}

impl Game {
    pub async fn start(mode: Mode, conn_kind: ConnKind, start_level: u32, conn: &mut Box<dyn ConnTrait>) -> Result<Self> {
        let seed_idx = conn_kind.is_host() as usize;

        let GameInfo { start_level, seeds } = GameInfo::sync(start_level, conn_kind, conn).await?;

        let player = Player::new(if matches!(mode, Mode::ComputerVsComputer) {
            PlayerKind::Ai
        } else {
            PlayerKind::Local
        }, start_level, seeds[seed_idx ^ 1]);

        let opponent = match mode {
            Mode::Singleplayer => {
                None
            },
            Mode::Multiplayer => {
                Some(Player::new(PlayerKind::Remote, start_level, seeds[seed_idx]))
            },
            Mode::PlayerVsComputer | Mode::ComputerVsComputer => {
                Some(Player::new(PlayerKind::Ai, start_level, thread_rng().gen::<u64>()))
            },
        };

        let mut game = Game { player, opponent };

        game.player.update_ghost();
        if let Some(opponent) = &mut game.opponent {
            opponent.update_ghost();
        }

        Ok(game)
    }
}

impl GameInfo {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.start_level.to_le_bytes());
        for s in &self.seeds {
            buf.extend_from_slice(&s.to_le_bytes());
        }
        buf
    }

    pub fn from_bytes(buf: [u8; 63]) -> GameInfo {
        let start_level_bytes: [u8; 4] = buf[0..4].try_into().unwrap();
        let start_level = u32::from_le_bytes(start_level_bytes);
        let p1_seed_bytes: [u8; 8] = buf[4..12].try_into().unwrap();
        let p1_seed = u64::from_le_bytes(p1_seed_bytes);
        let p2_seed_bytes: [u8; 8] = buf[12..20].try_into().unwrap();
        let p2_seed = u64::from_le_bytes(p2_seed_bytes);
        GameInfo { start_level, seeds: vec![p1_seed, p2_seed] }
    }

    async fn sync(start_level: u32, conn_kind: ConnKind, conn: &Box<dyn ConnTrait>) -> Result<GameInfo> {
        match conn_kind {
            ConnKind::Host => {
                let p1_seed = thread_rng().gen::<u64>();
                let p2_seed = thread_rng().gen::<u64>();
                let game_info = GameInfo { start_level, seeds: vec![p1_seed, p2_seed] };
                conn.send_info(&game_info).await?;
                Ok(game_info)
            },
            ConnKind::Client => {
                loop {
                    let (mode, payload) = conn.recv_tcp().await?;
                    match mode {
                        TcpPacketMode::Info => return Ok(GameInfo::from_bytes(payload)),
                        _ => continue,
                    }
                }
            },
            ConnKind::Empty => {
                let seed = thread_rng().gen::<u64>();
                Ok(GameInfo { start_level, seeds: vec![seed] })
            },
        }
    }
}

