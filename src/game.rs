use std::{io::Result, ops::Index};
use rand::{thread_rng, Rng};

use crate::{conn::{ConnKind, ConnTrait, TcpPacketMode}, debug_log, player::{Player, PlayerKind}};

pub struct Players {
    pub local: Player,
    pub remote: Option<Player>,
}

pub struct Game {
    pub players: Players,
}

pub struct GameInfo {
    pub start_level: u32,
    pub seeds: Vec<u64>,
}

impl Index<usize> for Players {
    type Output = Player;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.local,
            1 => &self.remote.as_ref().unwrap_or_else(|| panic!("Multiplayer not enabled")),
            _ => panic!("Index out of bounds"),
        }
    }
}

impl Game {
    pub async fn start(conn_kind: ConnKind, start_level: u32, conn: &mut Box<dyn ConnTrait>) -> Result<Self> {
        let seed_idx = conn_kind.is_host() as usize;

        let GameInfo { start_level, seeds } = GameInfo::sync(start_level, conn_kind, conn).await?;

        let local = Player::new(PlayerKind::Local, start_level, seeds[seed_idx ^ 1]);
        let remote = if conn_kind.is_multiplayer() {
            Some(Player::new(PlayerKind::Remote, start_level, seeds[seed_idx]))
        } else {
            None
        };

        let mut game = Game { players: Players { local, remote } };

        game.players.local.update_ghost();
        if let Some(remote) = &mut game.players.remote {
            remote.update_ghost();
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

