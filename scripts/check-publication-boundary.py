#!/usr/bin/env python3
from __future__ import annotations

import argparse
import ipaddress
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


EXCLUDED_PATHS = {"scripts/check-publication-boundary.py"}
SAFE_HOME_USERS = {"example", "local-user", "runner", "tester", "user"}
DOCUMENTATION_NETWORKS = tuple(
    ipaddress.ip_network(network)
    for network in ("192.0.2.0/24", "198.51.100.0/24", "203.0.113.0/24")
)


@dataclass(frozen=True, order=True)
class Finding:
    path: str
    line: int
    kind: str


def run(root: Path, command: list[str]) -> str:
    completed = subprocess.run(
        command,
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"{command[0]} failed with exit {completed.returncode}")
    return completed.stdout


def repository_files(root: Path) -> list[str]:
    try:
        return [path for path in run(root, ["jj", "file", "list"]).splitlines() if path]
    except (FileNotFoundError, RuntimeError):
        tracked = run(root, ["git", "ls-files", "-z"]).split("\0")
        untracked = run(
            root, ["git", "ls-files", "--others", "--exclude-standard", "-z"]
        ).split("\0")
        return [path for path in tracked + untracked if path]


def scan_text(relative: str, text: str) -> set[Finding]:
    findings: set[Finding] = set()
    home = re.compile(r"(?<![A-Za-z0-9_.-])/(?:Users|home)/([A-Za-z0-9._-]+)")
    windows_home = re.compile(r"(?i)(?<![A-Za-z0-9_.-])[A-Z]:\\Users\\([A-Za-z0-9._-]+)")
    private_host = re.compile(
        r"(?i)\b[a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\.[a-z0-9-]+)*\."
        r"(?:local|internal|lan|home\.arpa|ts\.net)\b(?!\.[a-z0-9])"
    )
    ipv4 = re.compile(r"(?<![0-9])(?:[0-9]{1,3}\.){3}[0-9]{1,3}(?![0-9])")
    secrets = [
        re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"),
        re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
        re.compile(r"\b(?:ghp|github_pat)_[A-Za-z0-9_]{20,}\b"),
    ]

    for line_number, line in enumerate(text.splitlines(), start=1):
        for match in home.finditer(line):
            if match.group(1).lower() not in SAFE_HOME_USERS:
                findings.add(Finding(relative, line_number, "machine-local-home-path"))
        for match in windows_home.finditer(line):
            if match.group(1).lower() not in SAFE_HOME_USERS:
                findings.add(Finding(relative, line_number, "machine-local-home-path"))
        if private_host.search(line):
            findings.add(Finding(relative, line_number, "private-hostname"))
        if any(pattern.search(line) for pattern in secrets):
            findings.add(Finding(relative, line_number, "secret-like-material"))
        for match in ipv4.finditer(line):
            try:
                address = ipaddress.ip_address(match.group(0))
            except ValueError:
                continue
            if address.is_loopback or address.is_unspecified:
                continue
            if any(address in network for network in DOCUMENTATION_NETWORKS):
                continue
            findings.add(Finding(relative, line_number, "specific-network-address"))
    return findings


def scan_repository(root: Path) -> set[Finding]:
    findings: set[Finding] = set()
    for relative in sorted(set(repository_files(root)) - EXCLUDED_PATHS):
        path = root / relative
        if not path.is_file():
            continue
        data = path.read_bytes()
        if b"\0" in data:
            continue
        findings.update(scan_text(relative, data.decode("utf-8", errors="ignore")))
    return findings


def self_test() -> int:
    unix_home = "/" + "/".join(("Users", "local-account", "src", "fdiff"))
    private_hostname = ".".join(("cache", "private", "internal"))
    private_address = str(ipaddress.ip_network("10.0.0.0/8").network_address + 12)
    private_key_header = "-----BEGIN " + "PRIVATE KEY-----"
    unsafe = {
        f"Built from {unix_home}.": "machine-local-home-path",
        f"Connect to {private_hostname}.": "private-hostname",
        f"Production is {private_address}.": "specific-network-address",
        private_key_header: "secret-like-material",
    }
    safe = [
        "Use <home>/<repo-root>.",
        "Use <home>/src/fdiff in examples.",
        "Documentation endpoint is 192.0.2.10.",
        "Local test binds to 127.0.0.1.",
        ".idea/**/dataSources.local.xml",
    ]
    for text, expected in unsafe.items():
        kinds = {finding.kind for finding in scan_text("fixture", text)}
        if expected not in kinds:
            print(f"self-test failed: missing {expected}", file=sys.stderr)
            return 1
    for text in safe:
        if scan_text("fixture", text):
            print("self-test failed: safe fixture was rejected", file=sys.stderr)
            return 1
    print("publication boundary self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Check tracked artifacts for private inventory")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()

    root = Path(__file__).resolve().parent.parent
    findings = sorted(scan_repository(root))
    if findings:
        for finding in findings:
            print(f"{finding.path}:{finding.line}: {finding.kind}", file=sys.stderr)
        print(f"publication boundary failed: {len(findings)} finding(s)", file=sys.stderr)
        return 1
    print("publication boundary passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
