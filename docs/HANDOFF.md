# fdiff Handoff

## Start Here

1. `AGENTS.md`와 `docs/agent-harness.md`를 읽는다.
2. `jj status`로 기존 변경을 확인한다.
3. `docs/status.md`와 `docs/roadmap.md`에서 현재 baseline과 첫 unchecked item을 찾는다.
4. 활성 `docs/todo-*.md`가 있으면 acceptance와 out-of-scope를 우선한다.
5. focused test 뒤 `scripts/check.sh`로 닫는다.

## Current Baseline

- Go prototype은 Rust single-crate application으로 전환되었다.
- 기본 interactive mode는 synchronized Commander-style dual-pane TUI이며 non-TTY에서는
  plain snapshot으로 fallback한다.
- diff core는 left-only, right-only, content change, type change, identical을 구분한다.
- folder hierarchy, expand/collapse, stable rescan selection, narrow stacked layout이
  구현되어 있다.
- 양쪽 regular file은 `Enter`로 bounded side-by-side line diff를 열 수 있고,
  binary/non-UTF-8 input은 byte summary로 fallback한다.
- 현재 다음 제품 slice는 status/path search와 status filter다.

## Architecture Map

- `src/main.rs`: CLI parsing, interactive/plain mode 선택, exit status.
- `src/diff.rs`: tree scan, semantic comparison, digest cache.
- `src/content.rs`: bounded file loading, text line alignment, binary summary.
- `src/tui.rs`: live loop, input, terminal lifecycle, dashboard rendering.
- `src/output.rs`: automation-friendly plain text rendering.
- `scripts/check.sh`: repository 전체 local gate.

## Completion Rule

분석이나 patch 적용만으로 완료하지 않는다. 사용자에게 보이는 mode를 실제로 실행하고,
focused test와 전체 gate를 통과한 뒤 status/roadmap을 현재 상태에 맞춘다. push, tag,
release는 별도 권한 경계다.
