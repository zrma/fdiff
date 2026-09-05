# Completed Milestones

## 2026-07-14: Rust TUI Foundation

- Go path-list prototype을 Rust folder diff engine으로 교체했다.
- file content, entry type, symlink target을 구분하는 semantic comparison을 추가했다.
- alternate screen 기반 live TUI, keyboard navigation, pause/refresh, plain fallback을
  추가했다.
- unit/render/smoke 검증과 GPT-5.6 agent-first repository contract를 추가했다.

검증 source of truth는 `scripts/check.sh`이며 세부 구현 상태는 `docs/status.md`가
소유한다.

## 2026-07-14: Canonical Harness Enforcement Alignment

- `agent-harness-v1` interface guard를 canonical 공통 구현과 byte-identical하게 맞췄다.
- generic public repository boundary guard의 visibility, repository identity, portfolio,
  external revision, local path/host/address 검사를 모두 적용했다.
- `.gitignore`를 gitignore.io의 Rust/editor/OS template과 fdiff application overlay로
  재생성하고 executable용 `Cargo.lock` 추적을 유지했다.

## Interactive Tree Navigation and Bounded Content Diff

동일한 relative path를 좌우 같은 row에 놓는 dual-pane tree를 구현했다. directory
expand/collapse와 subtree count, rescan 뒤 선택 경로·collapse state 복원, 좁은 terminal의
stacked layout을 제공한다. folder selection에서 `Enter`로 content diff를 열고
`Esc`/`Backspace`로 같은 선택에 돌아온다. plain/check와 diff classification은 유지했다.

text는 side-by-side line alignment와 row/page/change/horizontal navigation을 제공한다.
`similar` Myers 비교는 250 ms로 제한하고 file당 4 MiB를 넘으면 metadata만 표시한다.
binary/non-UTF-8 input은 첫 차이 byte와 bounded hex summary로 처리하고 terminal control
문자를 직접 전달하지 않는다. 이 제한은 interactive 응답성과 안전한 terminal 표시를
보존하기 위한 것이다. 현행 구현은 `src/content.rs`와 `src/tui.rs`가 소유한다.

구현 당시 wide/narrow render, representative corpus, 실제 PTY keyboard/terminal restore와
`scripts/check.sh`를 검증했다. 후속 search/filter, ignore/incremental scan, rename/export와
고급 content 표현은 `docs/roadmap.md`와 `docs/status.md`의 범위로 남긴다.
