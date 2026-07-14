#!/usr/bin/env python3
from __future__ import annotations

import argparse
import ipaddress
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


SAFE_HOME_USERS = {
    "example",
    "local-user",
    "me",
    "runner",
    "tester",
    "user",
    "you",
}

SAFE_KUBERNETES_CONTEXTS = {
    "example",
    "example-cluster",
    "kind",
    "minikube",
    "test",
    "test-cluster",
}

DOCUMENTATION_NETWORKS = tuple(
    ipaddress.ip_network(network)
    for network in ("192.0.2.0/24", "198.51.100.0/24", "203.0.113.0/24")
)

SAFE_NETWORK_LITERALS = {
    "10.0.0.0/8",
    "100.64.0.0/10",
    "172.16.0.0/12",
    "192.168.0.0/16",
}

RAW_EVIDENCE_NAME = (
    r"(?:[a-z0-9][a-z0-9._-]*[-_])?"
    r"(?:healthcheck|diagnostic|support-bundle|cluster-dump)[-_]"
    r"[0-9]{8}(?:[-_][0-9]{4,6})?"
)
RAW_EVIDENCE_PATH = re.compile(rf"(?i)(?:^|/){RAW_EVIDENCE_NAME}(?:/|$)")
RAW_EVIDENCE_REFERENCE = re.compile(
    rf"(?i)(?<![a-z0-9._/-]){RAW_EVIDENCE_NAME}(?:/|$)"
)


@dataclass(frozen=True, order=True)
class Finding:
    path: str
    line: int
    kind: str


def run_command(root: Path, command: list[str]) -> str:
    completed = subprocess.run(
        command,
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip().splitlines()
        summary = detail[-1] if detail else f"exit {completed.returncode}"
        raise RuntimeError(f"{command[0]} {command[1] if len(command) > 1 else ''} failed: {summary}")
    return completed.stdout


def run_git(root: Path, *args: str) -> str:
    return run_command(root, ["git", *args])


def run_jj(root: Path, *args: str) -> str:
    return run_command(root, ["jj", *args])


def repository_root(cwd: Path) -> Path:
    try:
        return Path(run_git(cwd, "rev-parse", "--show-toplevel").strip())
    except RuntimeError:
        return Path(run_jj(cwd, "workspace", "root").strip())


def tracked_files(root: Path) -> list[str]:
    try:
        return [item for item in run_git(root, "ls-files", "-z").split("\0") if item]
    except RuntimeError:
        return [item for item in run_jj(root, "file", "list").splitlines() if item]


def repository_identity(root: Path) -> tuple[str, str]:
    try:
        remote = run_git(root, "config", "--get", "remote.origin.url").strip()
    except RuntimeError:
        git_root = Path(run_jj(root, "git", "root").strip())
        remote = run_command(root, ["git", "-C", str(git_root), "config", "--get", "remote.origin.url"]).strip()
    match = re.search(r"(?:github\.com[/:])([^/]+)/([^/#]+?)(?:\.git)?$", remote)
    if not match:
        raise RuntimeError("origin does not identify a GitHub owner/repository")
    return match.group(1), match.group(2)


def live_visibility() -> str | None:
    explicit = os.environ.get("PUBLICATION_LIVE_VISIBILITY", "").strip().lower()
    if explicit:
        if explicit in {"public", "private", "internal"}:
            return "public" if explicit == "public" else "internal"
        raise RuntimeError("PUBLICATION_LIVE_VISIBILITY must be public, private, or internal")

    event_path = os.environ.get("GITHUB_EVENT_PATH")
    if not event_path:
        return None
    payload = json.loads(Path(event_path).read_text(encoding="utf-8"))
    repository = payload.get("repository") or {}
    visibility = str(repository.get("visibility") or "").lower()
    if visibility:
        return "public" if visibility == "public" else "internal"
    if "private" in repository:
        return "internal" if repository["private"] else "public"
    return None


def publication_class(root: Path) -> str:
    document = (root / "docs" / "agent-harness.md").read_text(encoding="utf-8")
    matches = re.findall(r"^- Publication class: `(public|internal)`\.$", document, flags=re.MULTILINE)
    if len(matches) != 1:
        raise RuntimeError("docs/agent-harness.md must declare exactly one publication class")
    expected_check = "- Publication boundary check: `scripts/check-publication-boundary.py`."
    if document.count(expected_check) != 1:
        raise RuntimeError("docs/agent-harness.md must declare the canonical publication boundary check")
    return matches[0]


def text_files(root: Path) -> Iterable[tuple[str, str]]:
    files = set(tracked_files(root))
    try:
        files.update(item for item in run_git(root, "ls-files", "--others", "--exclude-standard", "-z").split("\0") if item)
    except RuntimeError:
        pass
    for relative in sorted(files):
        path = root / relative
        if not path.is_file():
            continue
        data = path.read_bytes()
        if b"\0" in data:
            continue
        yield relative, data.decode("utf-8", errors="ignore")


def fixed_patterns(owner: str, repository: str) -> list[tuple[str, re.Pattern[str]]]:
    return [
        (
            "portfolio-disclosure",
            re.compile(
                r"(?i)(?:\b[0-9]+\s*(?:repositories|repos)\b|[0-9]+개\s*저장소|"
                r"all\s+repositories|cross-repository\s+agent-harness)"
            ),
        ),
        (
            "cross-repository-revision",
            re.compile(r"(?i)\b(?:gitops|rollout|cleanup|deployment)\s+(?:commit|revision|rev)\s+[`'\"]?[0-9a-f]{7,40}\b"),
        ),
        (
            "cross-repository-revision",
            re.compile(r"(?i)--(?:rollout|cleanup)-revision\s+[0-9a-f]{7,40}\b"),
        ),
        (
            "local-repository-state",
            re.compile(
                r"(?i)\b(?:companion|sibling)\b.{0,48}"
                r"\b(?:repo|repository)\b.{0,48}"
                r"\b(?:local|draft|branch|worktree)\b"
            ),
        ),
        (
            "same-owner-repository-url",
            re.compile(rf"(?i)(?:https?://github\.com/|git@github\.com:){re.escape(owner)}/(?!{re.escape(repository)}(?:\.git)?(?![A-Za-z0-9_.-]))[A-Za-z0-9_.-]+"),
        ),
        (
            "same-owner-repository-identity",
            re.compile(rf"(?i)(?<![A-Za-z0-9_./\\-]){re.escape(owner)}/(?!{re.escape(repository)}(?:\.git)?(?![A-Za-z0-9_.-]))[A-Za-z0-9_.-]+"),
        ),
    ]


def scan_text(
    relative: str,
    text: str,
    patterns: list[tuple[str, re.Pattern[str]]],
    top_levels: set[str],
) -> set[Finding]:
    findings: set[Finding] = set()
    path_pattern = re.compile(r"(?<![A-Za-z0-9_.<>-])([A-Za-z0-9_.-]+)/(?=(?:apps|manifests|argocd|common|infra|deploy|charts)/)", re.IGNORECASE)
    home_pattern = re.compile(r"(?<![A-Za-z0-9_.-])/(?:Users|home)/([A-Za-z0-9._-]+)")
    windows_home_pattern = re.compile(r"(?i)(?<![A-Za-z0-9_.-])[A-Z]:\\Users\\([A-Za-z0-9._-]+)")
    ipv4_pattern = re.compile(r"(?<![0-9])(?:[0-9]{1,3}\.){3}[0-9]{1,3}(?![0-9])")
    kubernetes_context_pattern = re.compile(r"(?i)--context(?:=|\s+)[`'\"]?([a-z0-9][a-z0-9._-]*)")
    private_hostname_pattern = re.compile(
        r"(?i)\b[a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\.[a-z0-9-]+)*\."
        r"(?:local|internal|lan|home\.arpa|ts\.net)\b"
    )
    record_like = Path(relative.split("@", 1)[0]).suffix.lower() in {".log", ".md", ".txt"}
    if RAW_EVIDENCE_PATH.search(relative):
        findings.add(Finding(relative, 1, "raw-runtime-evidence-path"))
    for match in RAW_EVIDENCE_REFERENCE.finditer(text):
        line_no = text.count("\n", 0, match.start()) + 1
        findings.add(Finding(relative, line_no, "raw-runtime-evidence-reference"))
    for kind, pattern in patterns:
        for match in pattern.finditer(text):
            line_no = text.count("\n", 0, match.start()) + 1
            findings.add(Finding(relative, line_no, kind))
    for line_no, line in enumerate(text.splitlines(), start=1):
        for match in path_pattern.finditer(line):
            if match.group(1) not in top_levels:
                findings.add(Finding(relative, line_no, "external-repository-path"))
        for match in home_pattern.finditer(line):
            if match.group(1).lower() not in SAFE_HOME_USERS:
                findings.add(Finding(relative, line_no, "machine-local-home-path"))
        for match in windows_home_pattern.finditer(line):
            if match.group(1).lower() not in SAFE_HOME_USERS:
                findings.add(Finding(relative, line_no, "machine-local-home-path"))
        for match in kubernetes_context_pattern.finditer(line):
            if match.group(1).lower() not in SAFE_KUBERNETES_CONTEXTS:
                findings.add(Finding(relative, line_no, "machine-kubernetes-context"))
        if record_like and private_hostname_pattern.search(line):
            findings.add(Finding(relative, line_no, "private-operations-hostname"))
        if record_like:
            for match in ipv4_pattern.finditer(line):
                try:
                    address = ipaddress.ip_address(match.group(0))
                except ValueError:
                    continue
                suffix = re.match(r"/[0-9]{1,3}", line[match.end() :])
                if suffix and f"{address}{suffix.group(0)}" in SAFE_NETWORK_LITERALS:
                    continue
                if address.is_loopback or address.is_unspecified or any(address in network for network in DOCUMENTATION_NETWORKS):
                    continue
                findings.add(Finding(relative, line_no, "specific-network-address"))
    return findings


def check_tree(root: Path, owner: str, repository: str) -> set[Finding]:
    top_levels = {path.split("/", 1)[0] for path in tracked_files(root)}
    patterns = fixed_patterns(owner, repository)
    findings: set[Finding] = set()
    for relative, text in text_files(root):
        findings.update(scan_text(relative, text, patterns, top_levels))
    return findings


def self_test() -> int:
    patterns = fixed_patterns("example", "public-app")
    top_levels = {"docs", "scripts", "src"}
    private_repository = "-".join(("private", "source"))
    private_revision = "".join(("dead", "beef"))
    local_state = " ".join(
        (
            "The companion platform",
            "repo currently has",
            "a local draft.",
        )
    )
    unix_home = "/" + "/".join(("Users", "local-account", "src", "public-app"))
    windows_home = "C:\\" + "\\".join(("Users", "local-account", "src", "public-app"))
    private_address = str(ipaddress.ip_network("100.64.0.0/10").network_address + 10)
    private_context = "-".join(("private", "cluster"))
    private_hostname = ".".join(("node-a", "private", "internal"))
    evidence_path = f"cluster-healthcheck-{'0' * 8}-{'0' * 6}/SUMMARY.txt"
    unsafe = [
        ("fixture", f"See https://github.com/example/{private_repository} for details."),
        ("fixture", f"Apply {private_repository}" + "/apps/service/manifests."),
        ("fixture", f"GitOps revision {private_revision} was promoted."),
        ("fixture", local_state),
        ("fixture", f"Built from {unix_home}."),
        ("fixture", f"Built from {windows_home}."),
        ("docs/HANDOFF.md", f"The target was {private_address}."),
        ("docs/HANDOFF.md", f"Run kubectl --context {private_context} get pods."),
        ("docs/HANDOFF.md", f"Connect to {private_hostname}."),
        (evidence_path, "ready"),
        ("docs/HANDOFF.md", f"Read {evidence_path} before publishing."),
    ]
    safe = [
        ("fixture", "See https://github.com/example/public-app/releases."),
        ("fixture", "The private deployment source of truth owns promotion."),
        ("fixture", "Use docs/deploy/checklist.md for the local contract."),
        ("fixture", "Use <home>/<repo-root> and <private-host>."),
        ("docs/HANDOFF.md", "Use 192.0.2.10 in documentation."),
        ("docs/HANDOFF.md", "The shared carrier-grade network is 100.64.0.0/10."),
        ("docs/HANDOFF.md", "Run kubectl --context example-cluster get pods."),
    ]
    if any(not scan_text(path, text, patterns, top_levels) for path, text in unsafe):
        print("self-test failed: expected unsafe fixture was not detected", file=sys.stderr)
        return 1
    if any(scan_text(path, text, patterns, top_levels) for path, text in safe):
        print("self-test failed: safe fixture was rejected", file=sys.stderr)
        return 1
    checker_path = Path(__file__)
    checker_findings = scan_text(
        "scripts/check-publication-boundary.py",
        checker_path.read_text(encoding="utf-8"),
        patterns,
        top_levels,
    )
    if checker_findings:
        print("self-test failed: checker source violates its own publication boundary", file=sys.stderr)
        return 1
    print("publication boundary repository gate self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate the repository-owned publication boundary contract.")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--stdin", action="store_true", help="scan candidate text from stdin instead of the working tree")
    parser.add_argument("--label", default="candidate", help="redacted location label used with --stdin")
    args = parser.parse_args()
    if args.self_test:
        return self_test()

    try:
        root = repository_root(Path.cwd())
        declared = publication_class(root)
        live = live_visibility()
        if live is not None and live != declared:
            print(
                f"publication boundary check failed: declared class {declared} does not match live class {live}",
                file=sys.stderr,
            )
            return 1
        if declared == "internal":
            print("publication boundary check passed: class=internal")
            return 0

        owner, repository = repository_identity(root)
        if args.stdin:
            top_levels = {path.split("/", 1)[0] for path in tracked_files(root)}
            findings = scan_text(args.label, sys.stdin.read(), fixed_patterns(owner, repository), top_levels)
        else:
            findings = check_tree(root, owner, repository)
        if findings:
            for finding in sorted(findings):
                print(
                    f"publication boundary finding: path={finding.path} line={finding.line} class={finding.kind}",
                    file=sys.stderr,
                )
            print(f"publication boundary check failed: {len(findings)} redacted finding(s)", file=sys.stderr)
            return 1
        print("publication boundary check passed: class=public")
        return 0
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"publication boundary check could not prove safety: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
