# fdiff Agent Guide

이 파일은 짧은 bootstrap map이다. 공통 실행 계약과 fdiff 상세 운영 규칙은
repository-owned 문서가 소유한다.

## First Read

- 공통 하네스 인터페이스와 fdiff overlay: `docs/agent-harness.md`.
- 현재 상태와 다음 순서: `docs/HANDOFF.md`, `docs/status.md`, `docs/roadmap.md`.
- 현재 작업: 활성 `docs/todo-*.md`.

<!-- agent-harness-baseline:start -->
## Agent Harness Baseline (GPT-5.6)

Baseline ID: `openai-gpt-5.6-2026-07-11`.

- Source of truth: use the `openai-docs` skill and the official [latest model guide](https://developers.openai.com/api/docs/guides/latest-model) plus [prompting best practices](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices) before changing OpenAI model, API, prompt, or agent guidance.
- Model target: when the task asks for the current or latest OpenAI baseline, use `gpt-5.6`. This is harness guidance, not proof that the application calls OpenAI; change runtime model strings only at an existing OpenAI integration point.
- Prompt budget: start with the smallest prompt and task-relevant tool set that reliably completes the work. Preserve project-specific constraints, remove redundant generic instructions, and add examples only for an observed failure.
- Request modes: for answer, explain, review, diagnose, or plan requests, inspect and report without implementation. For change, build, or fix requests, make the requested in-scope local changes and run relevant non-destructive validation.
- Permissions: reading, searching, editing in-scope files, and running non-destructive checks are pre-authorized for change tasks. Require confirmation for external writes not explicitly requested, destructive or irreversible actions, purchases or cost, secrets, or material scope expansion.
- Persistence: continue until the requested outcome is complete; do not stop after only analysis, a partial patch, or an intermediate tool success. Stop and escalate only at a real permission, product-decision, or external-state boundary.
- Verification: treat tool and patch success as provisional. Re-read the diff and verify the user-visible or runtime outcome with the narrowest meaningful checks, then broaden only when risk warrants it.
- Publication boundary: before a public push, tag/release, visibility change, or published-history rewrite, run the repository boundary check and any authorized local private-inventory check. Keep private inventory outside published repositories and CI configuration; retain only non-identifying responsibility boundaries and operational contracts.
- Tracked-artifact privacy: treat tool output, memory-derived environment context, local absolute paths, machine/host/cluster identifiers, internal endpoints or addresses, and full diagnostic logs as local-only by default. Do not paste raw stdout or stderr into tracked files; retain repository-owned decisions and redacted verification outcomes with placeholders such as `<repo-root>`, `<private-host>`, `<internal-ip>`, and `<cluster-context>`.
- Output: lead with the conclusion. Include required evidence, material caveats, and the next action; trim introductions, repetition, generic reassurance, and optional background before trimming required content.
- Structure: use a lightweight task-specific plan or output shape. Do not impose a global template or long process narration when the repository already supplies the necessary workflow.
- Modes and orchestration: configure Pro mode in the API or runtime rather than asking the model to “think harder.” Use Programmatic Tool Calling only for bounded reduction stages with explicit schemas, limits, and no approval-sensitive side effects; keep semantic decisions and final validation direct.
- Evaluation: add or retain harness instructions only when repository checks or representative tasks show they improve final-answer completeness, evidence quality, reliability, latency, or cost. Evaluate the final result, not just tool-call count.
- Project overlay: the remaining sections of this file and the linked project docs define domain-specific architecture, tests, safety boundaries, escalation rules, and publish gates. They may specialize this baseline but must not silently weaken its permission or evidence requirements.
<!-- agent-harness-baseline:end -->

## Project Overlay

- folder diff semantics는 `src/diff.rs`와 해당 unit test를 source of truth로 사용한다.
- TUI 변경은 terminal restore, narrow layout, keyboard navigation, non-TTY plain fallback을 함께 검증한다.
- 기본 전체 검증은 `scripts/check.sh`; 공개 경계는 `scripts/check-publication-boundary.py`로 확인한다.
- 로컬 VCS는 `jj`를 사용하고 push는 명시적 권한이 있을 때만 수행한다.
