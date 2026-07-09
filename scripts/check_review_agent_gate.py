#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


MANDATORY_ROLES = ("pr_reviewer", "functional_qa", "security_qa")
FORGE_GATE_EQUIVALENTS = ("forge_reviewer", "qax2", "orchestrator")
DESIGN_ROLE = "design_qa"
OK = "REVIEW_AGENT_OK"
NOT_APPLICABLE = "not_applicable"
PLACEHOLDER_VALUES = {
    "",
    "-",
    "n/a",
    "na",
    "none",
    "null",
    "not applicable",
    "not_applicable",
    "なし",
    "未記入",
}
BLOCKING_STATUS_WORDS = {"hold", "fail", "failed", "failure", "pending"}


class GateError(Exception):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Review Agent Gate を review packet から検証する。"
    )
    parser.add_argument(
        "--packet-path",
        type=Path,
        help="検証する review packet path。",
    )
    parser.add_argument(
        "--pr-number",
        help="artifacts/review_packets/pr-<PR番号>.md を検証する。",
    )
    return parser.parse_args()


def packet_path_from_args(args: argparse.Namespace) -> Path:
    if args.packet_path:
        return args.packet_path
    if args.pr_number:
        return Path("artifacts") / "review_packets" / f"pr-{args.pr_number}.md"
    raise GateError("--packet-path または --pr-number が必要です")


def extract_gate_section(markdown: str) -> str:
    match = re.search(r"^## REVIEW_AGENT_GATE\b.*$", markdown, flags=re.MULTILINE)
    if not match:
        raise GateError("REVIEW_AGENT_GATE section がありません")
    start = match.end()
    next_heading = re.search(r"^##\s+", markdown[start:], flags=re.MULTILINE)
    end = start + next_heading.start() if next_heading else len(markdown)
    section = markdown[start:end].strip()
    if not section:
        raise GateError("REVIEW_AGENT_GATE section が空です")
    return section


def normalize_cell(cell: str) -> str:
    return cell.strip().strip("`").strip()


def normalize_token(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", value.strip().strip("`").lower()).strip("_")


def status_kind(status: str) -> str:
    normalized = normalize_token(status)
    if normalized == normalize_token(OK):
        return "ok"
    if normalized == normalize_token(NOT_APPLICABLE):
        return "not_applicable"
    if any(word in normalized.split("_") for word in BLOCKING_STATUS_WORDS):
        return "blocking"
    return "other"


def is_placeholder(value: str) -> bool:
    normalized = re.sub(r"\s+", " ", normalize_cell(value).lower()).strip()
    compact = re.sub(r"[\s/._-]+", "", normalized).strip()
    tokenized = re.sub(r"[\s/._-]+", " ", normalized).strip(" .。、,;:!！")
    return (
        normalized.strip(" .。、,;:!！") in PLACEHOLDER_VALUES
        or compact in PLACEHOLDER_VALUES
        or tokenized in PLACEHOLDER_VALUES
    )


def parse_gate_rows(section: str) -> dict[str, dict[str, str]]:
    rows: dict[str, dict[str, str]] = {}
    header: list[str] | None = None
    for line in section.splitlines():
        stripped = line.strip()
        if not stripped.startswith("|") or not stripped.endswith("|"):
            continue
        cells = [normalize_cell(cell) for cell in stripped.strip("|").split("|")]
        if all(re.fullmatch(r":?-{3,}:?", cell) for cell in cells):
            continue
        lower_cells = [cell.lower() for cell in cells]
        if "role" in lower_cells and "status" in lower_cells:
            header = lower_cells
            continue
        if header is None:
            continue
        row = {header[index]: cells[index] for index in range(min(len(header), len(cells)))}
        role = row.get("role", "").strip()
        if role:
            if role in rows:
                raise GateError(f"{role} が重複しています")
            rows[role] = row
    if not rows:
        raise GateError("REVIEW_AGENT_GATE に role/status table がありません")
    return rows


def require_non_empty(row: dict[str, str], role: str, field: str) -> None:
    value = row.get(field, "")
    if is_placeholder(value):
        raise GateError(f"{role} の {field} が不足しています")


RISK_TIER_LOW_LINE = re.compile(r"^\s*RISK_TIER\s*:\s*low\b(?P<reason>.*)$", re.MULTILINE)


def risk_tier_low_reason(section: str) -> str | None:
    """packet の `RISK_TIER: low — <理由>` 行から理由を返す。無ければ None。"""
    match = RISK_TIER_LOW_LINE.search(section)
    if not match:
        return None
    reason = match.group("reason").strip().strip("-—:～ ").strip()
    if not reason or is_placeholder(reason):
        return None
    return reason


def require_merge_approval_not_granted(section: str) -> None:
    lines = [
        line
        for line in section.splitlines()
        if re.match(r"^\s*MERGE_APPROVAL\s*:", line)
    ]
    if not lines:
        raise GateError("MERGE_APPROVAL: not_granted がありません")
    match = re.fullmatch(r"\s*MERGE_APPROVAL\s*:\s*([A-Za-z0-9_-]+)\s*", lines[0])
    if len(lines) != 1 or not match or normalize_token(match.group(1)) != normalize_token("not_granted"):
        raise GateError("MERGE_APPROVAL は専用行で not_granted だけを指定してください")


def validate_gate(section: str) -> None:
    require_merge_approval_not_granted(section)

    rows = parse_gate_rows(section)
    for role, row in rows.items():
        status = row.get("status", "")
        if status_kind(status) == "blocking":
            raise GateError(f"{role} が {status} です")

    for role in MANDATORY_ROLES:
        row = rows.get(role)
        if row is None:
            raise GateError(f"必須 reviewer {role} がありません")
        if status_kind(row.get("status", "")) != "ok":
            raise GateError(f"{role} は {OK} でなければなりません")
        require_non_empty(row, role, "evidence")

    forge_ok = [
        role
        for role in FORGE_GATE_EQUIVALENTS
        if status_kind(rows.get(role, {}).get("status", "")) == "ok"
        and not is_placeholder(rows.get(role, {}).get("evidence", ""))
    ]
    if not forge_ok:
        # F4 比例ゲート: forge_reviewer 行が not_applicable の場合、packet に
        # `RISK_TIER: low` 行と理由が併記されているときのみ許容する
        # (design_qa の既存 not_applicable 許容パスと同型)。それ以外は従来どおり必須。
        forge_row = rows.get("forge_reviewer")
        risk_tier_reason = risk_tier_low_reason(section)
        if (
            forge_row is not None
            and status_kind(forge_row.get("status", "")) == "not_applicable"
            and risk_tier_reason is not None
            and not is_placeholder(forge_row.get("rationale", ""))
        ):
            pass  # risk_tier=low の比例緩和として許容 (merge gate 側で live 再検証される)
        else:
            raise GateError(
                "forge_reviewer、qax2、または orchestrator の REVIEW_AGENT_OK evidence が必要です"
                " (forge_reviewer not_applicable は RISK_TIER: low 行 + 理由がある場合のみ許容)"
            )

    design_row = rows.get(DESIGN_ROLE)
    if design_row is None:
        raise GateError("design_qa の OK または not_applicable rationale が必要です")
    design_status = status_kind(design_row.get("status", ""))
    if design_status == "ok":
        require_non_empty(design_row, DESIGN_ROLE, "evidence")
    elif design_status == "not_applicable":
        require_non_empty(design_row, DESIGN_ROLE, "rationale")
    else:
        raise GateError("design_qa は REVIEW_AGENT_OK または not_applicable でなければなりません")


def main() -> int:
    try:
        args = parse_args()
        path = packet_path_from_args(args)
        if not path.exists():
            raise GateError(f"review packet がありません: {path}")
        section = extract_gate_section(path.read_text(encoding="utf-8"))
        validate_gate(section)
    except GateError as error:
        print(f"review-agent-gate: fail: {error}", file=sys.stderr)
        return 1
    print("review-agent-gate: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
