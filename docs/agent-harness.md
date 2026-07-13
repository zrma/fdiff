# Agent Harness

## Interface

- Structure ID: `agent-harness-v1`.
- Baseline ID: `openai-gpt-5.6-2026-07-11`.
- Convergence stage: `canonical`.
- Target stage: `canonical`.
- Canonical check: `scripts/check-agent-harness-interface.sh`.
- Publication class: `public`.
- Publication boundary check: `scripts/check-publication-boundary.py`.

`AGENTS.md`가 공통 GPT-5.6 계약을 소유하고, 이 문서는 fdiff product/Rust/TUI
overlay와 현재 작업 문서로 가는 canonical 진입점이다.

Publication class는 현재 저장소 자체의 공개 경계만 선언한다. 공개 문서에는 실제
checkout 경로, 다른 저장소 inventory, 개인 hostname, 내부 endpoint/IP, local draft
상태를 남기지 않고 식별 불가능한 책임 경계와 repository-owned 판정만 기록한다.

Tracked artifact contract: raw tool output와 정확한 로컬 환경 evidence는 local-only로
취급한다. 공개 가능한 기록에는 repository-owned 결정, 필요한 명령 이름, redacted
검증 판정만 남기고 경로·호스트·주소·클러스터 값은 placeholder로 바꾼다.

## Project Objective

두 폴더의 구조와 파일 내용 차이를 즉시 이해할 수 있는 live terminal diff 경험을
제공한다. interactive terminal에서는 유려한 watch TUI를, automation과 pipe에서는
결정적 plain output과 exit status를 제공한다.

## Source Of Truth

- diff 의미와 cache: `src/diff.rs`; CLI mode/exit status: `src/main.rs`.
- TUI rendering과 input loop: `src/tui.rs`; plain output: `src/output.rs`.
- 현재 구현과 리스크: `docs/status.md`; 우선순위: `docs/roadmap.md`.
- 무컨텍스트 시작점: `docs/HANDOFF.md`; 현재 작업: 활성 `docs/todo-*.md`.
- 검증 선언: `docs/REPO_MANIFEST.yaml`과 `scripts/check.sh`.

## Autonomy And Permissions

- 목표와 acceptance가 명확한 로컬·가역 작업은 추가 승인 없이 구현, 검증,
  문서화, local `jj` change 정리까지 진행한다.
- 외부 write, secret, 비용, 파괴적 작업, 제품 방향 변경, published history rewrite,
  승인되지 않은 push는 에스컬레이션한다.
- 기존 사용자 변경을 보존하며 겹치는 변경은 실제 요구와 충돌할 때만 최소 범위로
  대체한다.

## Execution Loop

1. `jj status`, `docs/HANDOFF.md`, status/roadmap, 활성 todo를 확인한다.
2. diff core, CLI/output, TUI, harness 중 이번 논리 경계를 고정한다.
3. 재현 가능한 directory fixture, render test 또는 user-visible smoke를 먼저 정한다.
4. 가장 작은 기능 slice를 구현하고 focused test를 즉시 실행한다.
5. `scripts/check.sh`까지 넓혀 실패를 같은 루프에서 닫는다.
6. durable 상태만 status/roadmap/completed milestone 또는 todo에 반영한다.
7. 하나의 목적을 가진 `jj` change로 닫고 원격 write 전에는 승인을 받는다.

## Verification And Evidence

- 전체 local gate: `scripts/check.sh`.
- Rust gate: `cargo fmt --all --check`, `cargo test --all-targets --locked`,
  `cargo clippy --all-targets --locked -- -D warnings`.
- harness interface: `scripts/check-agent-harness-interface.sh`.
- publication boundary: `scripts/check-publication-boundary.py`; 공개 push 전에는 권한
  있는 machine-local private-inventory gate도 실행한다.
- TUI 변경은 `TestBackend` render regression과 실제 PTY smoke를 함께 확인한다.
- 최종 evidence에는 실행 명령, user-visible 결과, 남은 리스크, local/remote bookmark
  상태를 구분해 포함한다.

## Escalation

제품 semantics 선택, credential/private context, 비용, 파괴적 변경, 실제 terminal에서
재현할 수 없는 blocker, published history rewrite, 승인되지 않은 push가 필요할 때만
사용자에게 최소 판단을 요청한다. 구현 세부사항과 안전한 local 검증은 agent가 직접
결정한다.

## VCS And Publish

- 로컬 VCS는 `jj`를 사용하고 change description은 `<type>: <summary>`와 Codex
  attribution trailer 규칙을 따른다.
- 변경은 independently explainable하고 검증 가능한 milestone 단위로 유지한다.
- push/tag/release는 별도 외부-write 경계이며 명시적 권한 없이 실행하지 않는다.
- 공개 전에는 repository publication gate와 권한 있는 machine-local inventory gate를
  모두 통과한다.

## Harness Evaluation And Improvement

대표 folder fixture와 interactive session에서 완료성, diff 정확도, render 가독성,
input latency, scan latency, 회귀율을 평가한다. 반복 실패는 unit/render/smoke test,
검증 스크립트, concise operating rule 중 가장 가까운 계층에 기계화한다.

## Convergence

- `bridge`: 이 문서가 공통 인터페이스를 제공하고 기존 상세 문서를 연결한다.
- `normalized`: autonomy, execution, verification, escalation, VCS 정책을 동일 섹션
  계약으로 이동한다.
- `canonical`: 프로젝트 목적과 domain invariant는 local content로 유지하고 공통
  baseline, 제목 순서, 검사 골격을 잠근다.
- 단계 전환은 현재 저장소의 Structure ID, 섹션 순서, canonical check 결과로 검증하며 다른 저장소의 이름·개수·로컬 경로·공개 여부를 전제하지 않는다.

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
