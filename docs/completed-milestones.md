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
