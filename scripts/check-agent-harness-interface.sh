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

[ "$(grep -Fc '<!-- agent-harness-baseline:start -->' AGENTS.md || true)" -eq 1 ] ||
  fail "AGENTS.md must contain exactly one baseline start marker"
[ "$(grep -Fc '<!-- agent-harness-baseline:end -->' AGENTS.md || true)" -eq 1 ] ||
  fail "AGENTS.md must contain exactly one baseline end marker"

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
[ "$actual_agents_headings" = "$expected_agents_headings" ] ||
  fail "AGENTS.md section order differs from the compact agent-harness-v1 map"

for contract in \
  '- Structure ID: `agent-harness-v1`.' \
  '- Baseline ID: `openai-gpt-5.6-2026-07-11`.' \
  '- Convergence stage: `canonical`.' \
  '- Target stage: `canonical`.' \
  '- Canonical check: `scripts/check-agent-harness-interface.sh`.' \
  '- Publication class: `public`.' \
  '- Publication boundary check: `scripts/check-publication-boundary.py`.'; do
  grep -Fq -- "$contract" docs/agent-harness.md || fail "missing contract: $contract"
done
grep -Fq -- 'Tracked artifact contract:' docs/agent-harness.md ||
  fail "tracked-artifact contract is missing"

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
[ "$actual_headings" = "$expected_headings" ] ||
  fail "docs/agent-harness.md section order differs from agent-harness-v1"

if grep -Eiq 'GPT[- ]?5\.5|gpt-5\.5' AGENTS.md docs/agent-harness.md; then
  fail "active harness docs must not target GPT-5.5"
fi

scripts/check-publication-boundary.py
printf 'agent harness interface is valid: agent-harness-v1 / openai-gpt-5.6-2026-07-11\n'
