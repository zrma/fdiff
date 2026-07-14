# Product Roadmap

## P0: Rust TUI Foundation

- [x] Go prototype을 Rust single-crate application으로 전환
- [x] content-aware folder diff core 구현
- [x] live watch TUI와 keyboard navigation 구현
- [x] deterministic plain output과 `--check` exit status 구현
- [x] GPT-5.6 agent-first repository harness와 검증 gate 추가

## P1: Interactive Tree Experience

- [x] synchronized Commander-style left/right pane
- [x] 계층형 tree row와 expand/collapse navigation
- [ ] status/path search와 status filter
- [x] 좁은 terminal에서 responsive pane/detail layout
- [x] 선택 항목을 유지하는 stable rescan behavior

## P2: Scalable Watching

- [ ] ignore/exclude rule과 `.gitignore` 호환 정책
- [ ] filesystem notification 기반 incremental rescan
- [ ] parallel metadata/content scan과 bounded digest work
- [ ] unreadable path를 partial result warning으로 격리

## P3: Content Inspection

- [x] 선택 파일의 metadata/content preview
- [x] text line diff와 binary summary
- [ ] rename/move 후보 탐지
- [ ] machine-readable snapshot export

## P4: Distribution

- [ ] cross-platform PTY/CLI acceptance
- [ ] release artifact와 install flow
- [ ] shell completion과 manual page
- [ ] benchmark corpus와 performance budget
