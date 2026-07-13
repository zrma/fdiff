# Project Status

## Current Milestone

`v0.1.0` Rust/TUI foundation이 구현되어 있다.

- live terminal dashboard와 주기적 rescan
- path navigation, pause/resume, manual refresh, identical toggle
- content-aware file comparison과 digest cache
- symlink target/type-aware folder comparison
- plain output과 automation용 `--check` exit status
- GPT-5.6 agent-harness와 local/publication validation gates

## Known Limits

- flat path table이므로 큰 tree에서 계층 관계를 빠르게 파악하기 어렵다.
- ignore/exclude rule과 `.gitignore` integration이 없다.
- polling 기반이며 filesystem event notification을 사용하지 않는다.
- scan 중 하나의 unreadable path가 있으면 해당 scan 전체가 실패한다.
- content preview, line diff, rename detection, snapshot export가 없다.

## Next Slice

`docs/todo-interactive-tree-navigation.md`의 계층형 navigation을 먼저 구현한다. plain
output semantics와 diff core를 바꾸지 않고 TUI information architecture만 확장한다.
