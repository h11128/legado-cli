#!/usr/bin/env python3
"""afterMCPExecution: IDE save_source → claim deep_active + remind close-out.

Prefer `source-cli source push --file` for new drafts (claims + stderr recipe).
If the agent still uses MCP save_source, this hook arms the same gate.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path


def _roots(payload: dict) -> list[Path]:
    out: list[Path] = []
    if payload.get("cwd"):
        out.append(Path(payload["cwd"]))
    for r in payload.get("workspace_roots") or []:
        out.append(Path(r))
    return out


def _find_repo(payload: dict) -> Path | None:
    for r in _roots(payload):
        if (r / "crates" / "source-cli").is_dir():
            return r
        if (r / "temp" / "full_fix").is_dir() and (r / "config" / "mcp_defaults.json").is_file():
            return r
    return None


def _source_cli(repo: Path) -> Path | None:
    candidates = [
        repo / "crates" / "target" / "debug" / "source-cli.exe",
        repo / "crates" / "target" / "debug" / "source-cli",
        repo / "target" / "debug" / "source-cli.exe",
        repo / "target" / "debug" / "source-cli",
    ]
    for p in candidates:
        if p.is_file():
            return p
    # PATH fallback
    return Path("source-cli")


def _extract_url(tool_input) -> str | None:
    if tool_input is None:
        return None
    if isinstance(tool_input, str):
        try:
            tool_input = json.loads(tool_input)
        except json.JSONDecodeError:
            m = re.search(r'"bookSourceUrl"\s*:\s*"([^"]+)"', tool_input)
            return m.group(1) if m else None
    if not isinstance(tool_input, dict):
        return None
    src = tool_input.get("source")
    if isinstance(src, dict):
        u = src.get("bookSourceUrl")
        return str(u) if u else None
    if isinstance(src, str):
        try:
            obj = json.loads(src)
            u = obj.get("bookSourceUrl")
            return str(u) if u else None
        except json.JSONDecodeError:
            m = re.search(r'"bookSourceUrl"\s*:\s*"([^"]+)"', src)
            return m.group(1) if m else None
    return None


def main() -> int:
    raw = sys.stdin.read() or "{}"
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        print("{}")
        return 0

    name = (payload.get("tool_name") or payload.get("toolName") or "").lower()
    # Cursor may prefix MCP server: "legado-save_source" / "save_source"
    if "save_source" not in name.replace("-", "_") and name != "save_source":
        print("{}")
        return 0

    url = _extract_url(payload.get("tool_input") or payload.get("arguments"))

    # Prefer claim only when tool looks successful (status/error fields vary by Cursor).
    status = (payload.get("status") or "").lower()
    err = payload.get("error") or payload.get("tool_error")
    if status in ("error", "failed", "failure") or err:
        print(
            json.dumps(
                {
                    "additional_context": (
                        f"MCP save_source failed for {url or '(unknown)'} — not claiming deep_active. "
                        "Prefer `source-cli source push --file …`."
                    )
                },
                ensure_ascii=False,
            )
        )
        return 0

    if not url:
        print(
            json.dumps(
                {
                    "additional_context": (
                        "MCP save_source ran but bookSourceUrl was not parsed. "
                        "Prefer `source-cli source push --file …` so deep_active is claimed; "
                        "then ledger + retro close-out."
                    )
                },
                ensure_ascii=False,
            )
        )
        return 0

    repo = _find_repo(payload)
    claim_note = "mcp save_source"
    claim_ok = False
    claim_err = ""
    if repo is not None:
        cli = _source_cli(repo)
        try:
            r = subprocess.run(
                [
                    str(cli),
                    "closeout",
                    "claim",
                    "--url",
                    url,
                    "--note",
                    claim_note,
                ],
                cwd=str(repo),
                capture_output=True,
                text=True,
                timeout=30,
                env={**os.environ},
            )
            claim_ok = r.returncode == 0
            if not claim_ok:
                claim_err = (r.stderr or r.stdout or "")[:300]
        except (OSError, subprocess.TimeoutExpired) as e:
            claim_err = str(e)

    ctx = (
        f"MCP save_source for {url}. "
        f"deep_active claim={'ok' if claim_ok else 'FAILED: ' + claim_err}. "
        "Prefer `source-cli source push --file` next time (claims + recipe). "
        "Before next site: ledger append → retro append (trap/skill_fix/script_fix) → "
        "novel trap → skill+harness → git commit. "
        "Clear search throttle only via `source-cli check clear-cookies --url …`."
    )
    print(json.dumps({"additional_context": ctx}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
