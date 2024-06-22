# Tetris

Modern Tetris CLI written in Rust

![Preview](preview.png)

## Installation

```bash
git clone https://github.com/abusch8/Tetris
cd Tetris
make clean install
```

## Default Controls

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

- Leaderboard
- T-Spin scoring
- Fix soft drop scoring accuracy
- Line clear animation
- Scoring feedback
- Prevent infinity
