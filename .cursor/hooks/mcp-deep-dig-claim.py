#!/usr/bin/env python3
"""afterMCPExecution: IDE MCP deep-dig tools → claim deep_active + remind close-out.

Covers: save_source, debug_source, start_check_sources.
Prefer `source-cli source push` / `diagnose` / Python LegadoMcp (also claims).
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path


CLAIM_TOOLS = (
    "save_source",
    "debug_source",
    "start_check_sources",
)


def _roots(payload: dict) -> list[Path]:
    out: list[Path] = []
    if payload.get("cwd"):
        out.append(Path(payload["cwd"]))
    for r in payload.get("workspace_roots") or []:
        out.append(Path(r))
    env = (os.environ.get("LEGADO_SKILL_ROOT") or "").strip()
    if env:
        out.append(Path(env))
    out.append(Path("E:/Projects/legadoSkill"))
    return out


def _find_repo(payload: dict) -> Path | None:
    for r in _roots(payload):
        if (r / "crates" / "source-cli").is_dir():
            return r
        if (r / "temp" / "full_fix").is_dir() and (r / "config" / "mcp_defaults.json").is_file():
            # Prefer skill root when both legado and legadoSkill match markers
            if (r / "skills" / "legado-book-source-repair").is_dir():
                return r
    for r in _roots(payload):
        if (r / "temp" / "full_fix").is_dir() and (r / "config" / "mcp_defaults.json").is_file():
            return r
        sib = r.parent / "legadoSkill"
        if (sib / "crates" / "source-cli").is_dir():
            return sib
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
    return Path("source-cli")


def _tool_input(payload: dict):
    return payload.get("tool_input") or payload.get("arguments")


def _extract_url(tool_input, tool_name: str) -> str | None:
    if tool_input is None:
        return None
    if isinstance(tool_input, str):
        try:
            tool_input = json.loads(tool_input)
        except json.JSONDecodeError:
            m = re.search(r'"bookSourceUrl"\s*:\s*"([^"]+)"', tool_input)
            if m:
                return m.group(1)
            m = re.search(r'"url"\s*:\s*"(https?://[^"]+)"', tool_input)
            return m.group(1) if m else None
    if not isinstance(tool_input, dict):
        return None
    # debug_source / check
    u = tool_input.get("url")
    if isinstance(u, str) and u.startswith("http"):
        return u
    urls = tool_input.get("urls")
    if isinstance(urls, list) and urls:
        return str(urls[0])
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


def _norm_tool(name: str) -> str:
    n = (name or "").lower().replace("-", "_")
    for t in CLAIM_TOOLS:
        if t in n or n.endswith(t) or n == t:
            return t
    return ""


def main() -> int:
    raw = sys.stdin.read() or "{}"
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        print("{}")
        return 0

    name = payload.get("tool_name") or payload.get("toolName") or ""
    tool = _norm_tool(str(name))
    if not tool:
        print("{}")
        return 0

    url = _extract_url(_tool_input(payload), tool)

    status = (payload.get("status") or "").lower()
    err = payload.get("error") or payload.get("tool_error")
    if status in ("error", "failed", "failure") or err:
        print(
            json.dumps(
                {
                    "additional_context": (
                        f"MCP {tool} failed for {url or '(unknown)'} — not claiming deep_active."
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
                        f"MCP {tool} ran but URL was not parsed. "
                        "Run: source-cli closeout claim --url <bookSourceUrl> — then ledger+retro."
                    )
                },
                ensure_ascii=False,
            )
        )
        return 0

    repo = _find_repo(payload)
    claim_note = f"mcp {tool}"
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
                env={**os.environ, "LEGADO_SKILL_ROOT": str(repo)},
            )
            claim_ok = r.returncode == 0
            if not claim_ok:
                claim_err = (r.stderr or r.stdout or "")[:300]
        except (OSError, subprocess.TimeoutExpired) as e:
            claim_err = str(e)

    ctx = (
        f"MCP {tool} for {url}. "
        f"deep_active claim={'ok' if claim_ok else 'FAILED: ' + claim_err}. "
        "Before next site: ledger append → retro append (trap/skill_fix/script_fix) → "
        "novel trap → skill+harness or no_auto:<reason> → docs/source-repair-retrospective.md "
        "→ git commit."
    )
    print(json.dumps({"additional_context": ctx}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
