# Todo: Interactive Tree Navigation

Status: completed

## Goal

flat path table을 folder hierarchy가 보이는 tree navigation으로 바꿔 큰 directory
diff에서도 차이의 위치와 범위를 빠르게 이해할 수 있게 한다.

## Acceptance

- [x] directory/file depth가 indentation과 tree glyph로 드러난다.
- [x] directory row를 expand/collapse할 수 있다.
- [x] collapsed subtree의 visible row count가 directory row에 표시된다.
- [x] rescan 뒤에도 가능한 경우 선택 경로와 collapse state가 유지된다.
- [x] `--plain`, `--check`, diff classification semantics는 변하지 않는다.
- [x] wide/narrow `TestBackend` render test와 실제 PTY keyboard smoke가 통과한다.
- [x] `scripts/check.sh`가 통과한다.

## Delivered Design

- 동일한 relative path를 좌우의 같은 row에 배치하는 synchronized dual-pane
- 양쪽 pane의 synchronized selection highlight
- left-only, right-only, modified, type-changed를 pane 안쪽 marker로 표시
- 좁은 terminal에서 left/right pane을 위아래로 전환하는 responsive layout

## Out Of Scope

- text line diff와 content preview
- rename detection
- filesystem notification 기반 incremental scan
- ignore rule

## Suggested Order

1. `DiffReport`에서 parent/child relation을 계산하는 presentation model을 추가한다.
2. visible row flattening과 collapse state를 pure function으로 분리해 unit test한다.
3. TUI navigation과 rescan state restoration을 연결한다.
4. narrow/wide render와 실제 keyboard behavior를 검증한다.
