# Project Status

## Current Milestone

`v0.1.0` Rust/TUI foundation과 Commander-style tree navigation이 구현되어 있다.

- synchronized left/right pane과 주기적 rescan
- 계층형 folder row, expand/collapse, active pane focus
- stable path selection과 collapse state, narrow terminal stacked layout
- path navigation, pause/resume, manual refresh, identical toggle
- content-aware file comparison과 digest cache
- symlink target/type-aware folder comparison
- plain output과 automation용 `--check` exit status
- GPT-5.6 agent-harness와 local/publication validation gates
- canonical `agent-harness-v1` interface guard와 generic publication boundary guard
- gitignore.io Rust/editor/OS baseline과 fdiff local-artifact overlay

## Known Limits

- active pane은 현재 탐색 focus만 나타내며 pane별 file operation은 제공하지 않는다.
- ignore/exclude rule과 `.gitignore` integration이 없다.
- polling 기반이며 filesystem event notification을 사용하지 않는다.
- scan 중 하나의 unreadable path가 있으면 해당 scan 전체가 실패한다.
- content preview, line diff, rename detection, snapshot export가 없다.

## Publication Boundary

- repository-owned interface/publication guard는 canonical 공통 구현을 그대로 사용한다.
- 공개 push 전에는 repository gate와 별도로 권한 있는 machine-local private-inventory
  gate를 실행해야 한다.

## Next Slice

status/path search와 status filter를 추가한다. dual-pane row alignment, plain output
semantics, diff core는 유지한다.
