#!/usr/bin/env python3
"""Validate Epic Delivery Plan cross-artifact trace anchors."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

READY_STATES = {"ready", "running", "done"}


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_case_pr_links(plan: dict[str, Any]) -> list[str]:
    if plan.get("status") not in READY_STATES:
        return []

    cases_by_id = {
        item.get("case_id"): item
        for item in plan.get("case_graph", [])
        if isinstance(item, dict)
    }
    claim_ids = {
        item.get("claim_id")
        for item in plan.get("claim_tree", [])
        if isinstance(item, dict)
    }
    pr_by_id: dict[Any, Any] = {}
    errors: list[str] = []
    for pr in plan.get("pr_plan", []):
        if not isinstance(pr, dict):
            continue
        planned_pr = pr.get("planned_pr_id")
        if planned_pr in pr_by_id:
            errors.append(f"duplicate planned_pr_id {planned_pr}")
            continue
        pr_by_id[planned_pr] = pr
    for case in plan.get("case_graph", []):
        if not isinstance(case, dict):
            continue
        case_id = case.get("case_id")
        for dependency in case.get("depends_on", []):
            if dependency not in cases_by_id:
                errors.append(f"case {case_id} depends on missing case {dependency}")
        for claim_id in case.get("claim_ids", []):
            if claim_id not in claim_ids:
                errors.append(f"case {case_id} references missing claim_id {claim_id}")
        planned_pr = case.get("planned_pr")
        pr = pr_by_id.get(planned_pr)
        if pr is None:
            errors.append(f"case {case_id} references missing planned_pr {planned_pr}")
            continue
        if pr.get("case_id") != case_id:
            errors.append(
                f"case {case_id} references planned_pr {planned_pr}, "
                f"but that PR maps to case {pr.get('case_id')}"
            )
    for planned_pr, pr in pr_by_id.items():
        case_id = pr.get("case_id")
        case = cases_by_id.get(case_id)
        if case is None:
            errors.append(f"planned_pr {planned_pr} references missing case {case_id}")
            continue
        if case.get("planned_pr") != planned_pr:
            errors.append(
                f"planned_pr {planned_pr} maps to case {case_id}, "
                f"but that case references planned_pr {case.get('planned_pr')}"
            )
    for proof in plan.get("proof_strategy", []):
        if not isinstance(proof, dict):
            continue
        claim_id = proof.get("claim_id")
        if claim_id not in claim_ids:
            errors.append(f"proof_strategy references missing claim_id {claim_id}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("plan", type=Path)
    args = parser.parse_args()

    plan = load_json(args.plan)

    errors = validate_case_pr_links(plan)
    if errors:
        for error in errors:
            print(error)
        return 1

    print("epic delivery plan validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
