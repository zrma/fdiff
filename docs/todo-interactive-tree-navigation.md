# Todo: Interactive Tree Navigation

## Goal

flat path table을 folder hierarchy가 보이는 tree navigation으로 바꿔 큰 directory
diff에서도 차이의 위치와 범위를 빠르게 이해할 수 있게 한다.

## Acceptance

- directory/file depth가 indentation과 tree glyph로 드러난다.
- directory row를 expand/collapse할 수 있다.
- collapsed subtree의 difference count가 directory row에 표시된다.
- rescan 뒤에도 가능한 경우 선택 경로와 collapse state가 유지된다.
- `--plain`, `--check`, diff classification semantics는 변하지 않는다.
- wide/narrow `TestBackend` render test와 실제 PTY keyboard smoke가 통과한다.
- `scripts/check.sh`가 통과한다.

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
