use crate::player::Player;

#[derive(Copy, Clone)]
pub enum ClearType {
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

impl ClearType {
    pub fn from_state(player: &Player) -> Self {
        let num_cleared = player.clearing.len();
        let perfect_clear = player.stack.iter().flatten().all(|cell| cell.is_none());
        let is_t_spin = player.t_spin_check();
        match (num_cleared, perfect_clear, is_t_spin) {
            (_, true, _) => ClearType::PerfectClear,
            (1, false, false) => ClearType::Single,
            (2, false, false) => ClearType::Double,
            (3, false, false) => ClearType::Triple,
            (4, false, false) => ClearType::Tetris,
            (1, false, true) => ClearType::TSpinSingle,
            (2, false, true) => ClearType::TSpinDouble,
            (3, false, true) => ClearType::TSpinTriple,
            _ => panic!("Invalid clear type"),
        }
    }

    pub fn line_clear_count(self) -> usize {
        match self {
            ClearType::Single | ClearType::TSpinSingle => 1,
            ClearType::Double | ClearType::TSpinDouble => 2,
            ClearType::Triple | ClearType::TSpinTriple => 3,
            ClearType::Tetris => 4,
            ClearType::PerfectClear => 4,
        }
    }

    pub fn garbage_line_count(self) -> usize {
        match self {
            ClearType::Single => 0,
            ClearType::Double => 1,
            ClearType::Triple => 2,
            ClearType::Tetris => 4,
            ClearType::TSpinSingle => 2,
            ClearType::TSpinDouble => 4,
            ClearType::TSpinTriple => 6,
            ClearType::PerfectClear => 10,
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

    pub fn score_clear(&mut self, clear_type: ClearType) {
        let line_clear_count = clear_type.line_clear_count() as u32;
        self.lines += line_clear_count;
        self.level = self.start_level + self.lines / 10;
        self.combo += 1;
        self.score += match clear_type {
            ClearType::PerfectClear => {
                match line_clear_count {
                    1 => self.level * 800,
                    2 => self.level * 1200,
                    3 => self.level * 1800,
                    4 => self.level * 2000,
                    _ => 0,
                }
            },
            ClearType::Single => self.level * 100,
            ClearType::Double => self.level * 300,
            ClearType::Triple => self.level * 500,
            ClearType::Tetris => self.level * 800,
            ClearType::TSpinSingle => self.level * 800,
            ClearType::TSpinDouble => self.level * 1200,
            ClearType::TSpinTriple => self.level * 1600,

        };
        self.score += 50 * self.combo as u32 * self.level;
    }
}
