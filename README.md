# Tetris

Modern Tetris CLI written in Rust

## Installation

```bash
git clone https://github.com/abusch8/Tetris
cd Tetris
cargo build --release
```

## Controls

|Command            |Key            |
|-------------------|---------------|
|Clockwise          |`[↑]` / `[W]`  |
|Left               |`[←]` / `[A]`  |
|Soft-Drop          |`[↓]` / `[S]`  |
|Right              |`[→]` / `[D]`  |
|Hard-Drop          |`[SPACE]`      |
|Counter-Clockwise  |`[Z]`          |
|Hold               |`[C]`          |
|Quit               |`[ESC]` / `[Q]`|

## TODO

- Scoreboard
- T-Spin scoring
- Fix soft drop scoring accuracy
- Level fall speed/gravity accuracy
- Clear animation
- Scoring feedback
