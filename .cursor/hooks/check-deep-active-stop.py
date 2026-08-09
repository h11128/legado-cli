#!/usr/bin/env python3
"""Cursor stop hook: if deep_active is unsealed, auto-follow up to finish close-out.

Create (`source push`) and repair (`diagnose` / `oneshot` / LegadoMcp debug|save|check)
both claim temp/full_fix/deep_active.json (under legadoSkill). Agents must ledger +
retro append before the next site — do not wait for the user to remind.

Workspace may be `legado` while deep_active lives in sibling `legadoSkill` — search both.
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path


def _candidate_roots(payload: dict) -> list[Path]:
    roots: list[Path] = []
    for key in ("cwd",):
        v = payload.get(key)
        if v:
            roots.append(Path(v))
    for v in payload.get("workspace_roots") or []:
        roots.append(Path(v))
    env = (os.environ.get("LEGADO_SKILL_ROOT") or "").strip()
    if env:
        roots.append(Path(env))
    # Sibling / fixed skill roots (agents often open legado, not legadoSkill)
    extra: list[Path] = []
    for r in list(roots):
        extra.append(r.parent / "legadoSkill")
        if r.name == "legado":
            extra.append(r.parent / "legadoSkill")
    roots.extend(extra)
    roots.append(Path("E:/Projects/legadoSkill"))
    out: list[Path] = []
    seen: set[str] = set()
    for r in roots:
        try:
            key = str(r.resolve())
        except OSError:
            key = str(r)
        if key not in seen:
            seen.add(key)
            out.append(r)
    return out


def _find_active(payload: dict) -> Path | None:
    """Prefer a root that already has deep_active.json; else skill markers."""
    candidates = _candidate_roots(payload)
    for r in candidates:
        p = r / "temp" / "full_fix" / "deep_active.json"
        if p.is_file():
            return p
    for r in candidates:
        if (r / "crates" / "source-cli").is_dir() or (r / "skills" / "legado-book-source").is_dir():
            p = r / "temp" / "full_fix" / "deep_active.json"
            return p if p.is_file() else None
    return None


def main() -> int:
    raw = sys.stdin.read() or "{}"
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        print("{}")
        return 0

    if payload.get("status") == "aborted":
        print("{}")
        return 0

    try:
        loop_count = int(payload.get("loop_count") or 0)
    except (TypeError, ValueError):
        loop_count = 0
    if loop_count >= 3:
        # Cap auto-nudges; leftover claim needs explicit closeout release.
        print("{}")
        return 0

    active_path = _find_active(payload)
    if active_path is None:
        print("{}")
        return 0

    try:
        data = json.loads(active_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        print("{}")
        return 0

    if data.get("sealed") is True:
        print("{}")
        return 0

    url = data.get("url") or "(unknown url)"
    note = data.get("note") or ""
    msg = (
        f"deep_active is still unsealed for {url} (note={note}). "
        "Finish mandatory close-out before ending this turn "
        "(create/repair/manual LegadoMcp deep dig share this gate):\n"
        f"  source-cli ledger append --url '{url}' --step check --result '校验成功'|fail:…|skip:…\n"
        f"  source-cli retro append --url '{url}' --status fixed|skip|fail "
        "--trap '…' --skill-fix 0|1 --script-fix '…'\n"
        "Novel trap → update skill + harness (or script_fix=no_auto:<reason≥8>) "
        "+ short note in docs/source-repair-retrospective.md → git commit.\n"
        "Escape only: source-cli closeout release --url … --status skip"
    )
    print(json.dumps({"followup_message": msg}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
