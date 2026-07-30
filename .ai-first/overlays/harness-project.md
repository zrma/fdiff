## Project Overlay

- 같은 상대 경로의 일반 파일은 내용 digest로 판정하고 size/mtime은 cache
  invalidation에만 사용한다.
- symlink는 follow하지 않고 link target을 비교한다. 디렉터리는 child path가 별도
  entry이므로 양쪽에 존재하는 디렉터리 자체는 동일하다.
- scan 실패 시 마지막 정상 frame을 보존하고 오류를 표시한다.
- TUI 종료나 오류 시 raw mode, alternate screen, mouse capture를 반드시 복원한다.
- non-TTY와 `--plain` output은 ANSI control sequence 없이 결정적으로 유지한다.

## Related Documents

- Navigation: `docs/HANDOFF.md`.
- Current state and direction: `docs/status.md`, `docs/roadmap.md`.
- Completed work: `docs/completed-milestones.md`.
- Active work: `docs/todo-interactive-tree-navigation.md`.
- Declared checks: `docs/REPO_MANIFEST.yaml`.
