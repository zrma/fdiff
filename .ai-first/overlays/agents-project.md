## Repository Overlay

- folder diff semantics는 `src/diff.rs`와 해당 unit test를 source of truth로 사용한다.
- TUI 변경은 terminal restore, narrow layout, keyboard navigation, non-TTY plain fallback을 함께 검증한다.
- 기본 전체 검증은 `scripts/check.sh`; 공개 경계는 `scripts/check-publication-boundary.py`로 확인한다.
- 로컬 VCS는 `jj`를 사용하고 push는 명시적 권한이 있을 때만 수행한다.
