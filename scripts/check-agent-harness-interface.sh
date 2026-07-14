#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

fail() {
  printf 'agent harness interface check failed: %s\n' "$1" >&2
  exit 1
}

for required_file in AGENTS.md docs/agent-harness.md scripts/check-publication-boundary.py; do
  [ -s "$required_file" ] || fail "missing or empty $required_file"
done

baseline_start=$(grep -Fc '<!-- agent-harness-baseline:start -->' AGENTS.md || true)
baseline_end=$(grep -Fc '<!-- agent-harness-baseline:end -->' AGENTS.md || true)
[ "$baseline_start" -eq 1 ] || fail "AGENTS.md must contain exactly one baseline start marker"
[ "$baseline_end" -eq 1 ] || fail "AGENTS.md must contain exactly one baseline end marker"

grep -Fq 'Baseline ID: `openai-gpt-5.6-2026-07-11`.' AGENTS.md ||
  fail "AGENTS.md baseline ID is missing or stale"
grep -Fq 'docs/agent-harness.md' AGENTS.md ||
  fail "AGENTS.md must route to docs/agent-harness.md"
grep -Fq -- '- Tracked-artifact privacy:' AGENTS.md ||
  fail "AGENTS.md tracked-artifact privacy contract is missing"

expected_agents_headings=$(cat <<'HEADINGS'
## First Read
## Agent Harness Baseline (GPT-5.6)
## Project Overlay
HEADINGS
)
actual_agents_headings=$(sed -n 's/^\(## .*\)$/\1/p' AGENTS.md)

if [ "$actual_agents_headings" != "$expected_agents_headings" ]; then
  printf 'expected AGENTS.md headings:\n%s\n' "$expected_agents_headings" >&2
  printf 'actual AGENTS.md headings:\n%s\n' "$actual_agents_headings" >&2
  fail "AGENTS.md section order differs from the compact agent-harness-v1 map"
fi

grep -Fq -- '- Structure ID: `agent-harness-v1`.' docs/agent-harness.md ||
  fail "docs/agent-harness.md structure ID is missing or stale"
grep -Fq -- '- Baseline ID: `openai-gpt-5.6-2026-07-11`.' docs/agent-harness.md ||
  fail "docs/agent-harness.md baseline ID is missing or stale"
grep -Fq -- '- Convergence stage: `canonical`.' docs/agent-harness.md ||
  fail "docs/agent-harness.md convergence stage must be canonical"
grep -Fq -- '- Target stage: `canonical`.' docs/agent-harness.md ||
  fail "docs/agent-harness.md target stage must remain canonical"
grep -Fq -- '- Canonical check: `scripts/check-agent-harness-interface.sh`.' docs/agent-harness.md ||
  fail "docs/agent-harness.md canonical check path is missing or stale"
publication_class_count=$(grep -Ec '^- Publication class: `(public|internal)`\.$' docs/agent-harness.md || true)
[ "$publication_class_count" -eq 1 ] ||
  fail "docs/agent-harness.md must declare exactly one publication class"
grep -Fq -- '- Publication boundary check: `scripts/check-publication-boundary.py`.' docs/agent-harness.md ||
  fail "docs/agent-harness.md publication boundary check path is missing or stale"
grep -Fq -- 'Tracked artifact contract:' docs/agent-harness.md ||
  fail "docs/agent-harness.md tracked-artifact contract is missing"
grep -Fq -- '- 단계 전환은 현재 저장소의 Structure ID, 섹션 순서, canonical check 결과로 검증하며 다른 저장소의 이름·개수·로컬 경로·공개 여부를 전제하지 않는다.' docs/agent-harness.md ||
  fail "docs/agent-harness.md repository-boundary contract is missing or stale"

expected_headings=$(cat <<'HEADINGS'
## Interface
## Project Objective
## Source Of Truth
## Autonomy And Permissions
## Execution Loop
## Verification And Evidence
## Escalation
## VCS And Publish
## Harness Evaluation And Improvement
## Convergence
## Project Overlay
## Related Documents
HEADINGS
)
actual_headings=$(sed -n 's/^\(## .*\)$/\1/p' docs/agent-harness.md)

if [ "$actual_headings" != "$expected_headings" ]; then
  printf 'expected headings:\n%s\n' "$expected_headings" >&2
  printf 'actual headings:\n%s\n' "$actual_headings" >&2
  fail "docs/agent-harness.md section order differs from agent-harness-v1"
fi

if grep -Eiq 'GPT[- ]?5\.5|gpt-5\.5' AGENTS.md docs/agent-harness.md; then
  fail "active harness docs must not target GPT-5.5"
fi

if grep -Eiq '[0-9]+개 저장소|[0-9]+ repositories|all[[:space:]]+repositories|cross-repository .*agent-harness-v1|agent-harness-v1.*cross-repository' AGENTS.md ||
  find docs -type f -name '*.md' -exec grep -Eil '[0-9]+개 저장소|[0-9]+ repositories|all[[:space:]]+repositories|cross-repository .*agent-harness-v1|agent-harness-v1.*cross-repository' {} + | grep -q .; then
  fail "harness docs must not expose a repository portfolio"
fi

scripts/check-publication-boundary.py

printf 'agent harness interface is valid: agent-harness-v1 / openai-gpt-5.6-2026-07-11\n'
