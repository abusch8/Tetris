use crate::player::Player;

#[derive(Copy, Clone)]
pub enum ClearKind {
    PerfectClear,
    Single,
    Double,
    Triple,
    Tetris,
    TSpinSingle,
    TSpinDouble,
    TSpinTriple,
}

pub struct Score {
    pub start_level: u32,
    pub level: u32,
    pub lines: u32,
    pub score: u32,
    pub combo: i32,
}

impl ClearKind {
    pub fn from_state(player: &Player) -> Self {
        let num_cleared = player.clearing.len();
        let perfect_clear = player.stack.iter().flatten().all(|cell| cell.is_none());
        let is_t_spin = player.t_spin_check();
        match (num_cleared, perfect_clear, is_t_spin) {
            (_, true, _) => ClearKind::PerfectClear,
            (1, false, false) => ClearKind::Single,
            (2, false, false) => ClearKind::Double,
            (3, false, false) => ClearKind::Triple,
            (4, false, false) => ClearKind::Tetris,
            (1, false, true) => ClearKind::TSpinSingle,
            (2, false, true) => ClearKind::TSpinDouble,
            (3, false, true) => ClearKind::TSpinTriple,
            _ => panic!("Invalid clear type"),
        }
    }

    pub fn line_clear_count(self) -> usize {
        match self {
            ClearKind::Single | ClearKind::TSpinSingle => 1,
            ClearKind::Double | ClearKind::TSpinDouble => 2,
            ClearKind::Triple | ClearKind::TSpinTriple => 3,
            ClearKind::Tetris => 4,
            ClearKind::PerfectClear => 4,
        }
    }

    pub fn garbage_line_count(self) -> usize {
        match self {
            ClearKind::Single => 0,
            ClearKind::Double => 1,
            ClearKind::Triple => 2,
            ClearKind::Tetris => 4,
            ClearKind::TSpinSingle => 2,
            ClearKind::TSpinDouble => 4,
            ClearKind::TSpinTriple => 6,
            ClearKind::PerfectClear => 10,
        }
    }
}

impl Score {
    pub fn new(start_level: u32) -> Self {
        Score {
            start_level,
            level: start_level,
            lines: 0,
            score: 0,
            combo: -1,
        }
    }

    pub fn score_clear(&mut self, clear_type: ClearKind) {
        let line_clear_count = clear_type.line_clear_count() as u32;
        self.lines += line_clear_count;
        self.level = self.start_level + self.lines / 10;
        self.combo += 1;
        self.score += match clear_type {
            ClearKind::PerfectClear => {
                match line_clear_count {
                    1 => self.level * 800,
                    2 => self.level * 1200,
                    3 => self.level * 1800,
                    4 => self.level * 2000,
                    _ => 0,
                }
            },
            ClearKind::Single => self.level * 100,
            ClearKind::Double => self.level * 300,
            ClearKind::Triple => self.level * 500,
            ClearKind::Tetris => self.level * 800,
            ClearKind::TSpinSingle => self.level * 800,
            ClearKind::TSpinDouble => self.level * 1200,
            ClearKind::TSpinTriple => self.level * 1600,

        };
        self.score += 50 * self.combo as u32 * self.level;
    }
}
