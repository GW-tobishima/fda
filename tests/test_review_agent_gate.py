import importlib.util
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "scripts/check_review_agent_gate.py"

spec = importlib.util.spec_from_file_location("check_review_agent_gate", MODULE_PATH)
check_review_agent_gate = importlib.util.module_from_spec(spec)
assert spec and spec.loader
sys.modules[spec.name] = check_review_agent_gate
spec.loader.exec_module(check_review_agent_gate)


VALID_PACKET = """# PR Review Packet

## REVIEW_AGENT_GATE

MERGE_APPROVAL: not_granted

| role | status | evidence | rationale |
|---|---|---|---|
| pr_reviewer | REVIEW_AGENT_OK | pr reviewer evidence | correctness reviewed |
| functional_qa | REVIEW_AGENT_OK | functional evidence | AC reviewed |
| security_qa | REVIEW_AGENT_OK | security evidence | security reviewed |
| qax2 | REVIEW_AGENT_OK | qax2 evidence | ATO gate reviewed |
| design_qa | not_applicable | - | UI / visual 変更なし |

## NEXT

done
"""


class ReviewAgentGateTest(unittest.TestCase):
    def test_accepts_complete_review_agent_gate(self):
        section = check_review_agent_gate.extract_gate_section(VALID_PACKET)
        check_review_agent_gate.validate_gate(section)

    def test_rejects_missing_mandatory_reviewer(self):
        packet = VALID_PACKET.replace("| security_qa | REVIEW_AGENT_OK | security evidence | security reviewed |\n", "")
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "security_qa"):
            check_review_agent_gate.validate_gate(section)

    def test_rejects_hold_status(self):
        packet = VALID_PACKET.replace(
            "| functional_qa | REVIEW_AGENT_OK | functional evidence | AC reviewed |",
            "| functional_qa | REVIEW_AGENT_HOLD | blocker | AC reviewed |",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "functional_qa"):
            check_review_agent_gate.validate_gate(section)

    def test_rejects_pending_status_even_when_alternate_reviewer_is_ok(self):
        packet = VALID_PACKET.replace(
            "| qax2 | REVIEW_AGENT_OK | qax2 evidence | ATO gate reviewed |",
            "| forge_reviewer | REVIEW_AGENT_PENDING | pending evidence | pending |\n"
            "| qax2 | REVIEW_AGENT_OK | qax2 evidence | ATO gate reviewed |",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "forge_reviewer"):
            check_review_agent_gate.validate_gate(section)

    def test_rejects_duplicate_role_even_when_later_row_is_ok(self):
        packet = VALID_PACKET.replace(
            "| qax2 | REVIEW_AGENT_OK | qax2 evidence | ATO gate reviewed |",
            "| qax2 | REVIEW_AGENT_HOLD | blocker | blocker remains |\n"
            "| qax2 | REVIEW_AGENT_OK | qax2 evidence | ATO gate reviewed |",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "qax2"):
            check_review_agent_gate.validate_gate(section)

    def test_accepts_orchestrator_as_forge_gate_equivalent(self):
        packet = VALID_PACKET.replace(
            "| qax2 | REVIEW_AGENT_OK | qax2 evidence | ATO gate reviewed |",
            "| orchestrator | REVIEW_AGENT_OK | orchestrator evidence | review-gate run |",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        check_review_agent_gate.validate_gate(section)

    def test_rejects_merge_approval_granted_even_if_not_granted_is_mentioned(self):
        packet = VALID_PACKET.replace(
            "MERGE_APPROVAL: not_granted",
            "MERGE_APPROVAL: granted\n\n補足: MERGE_APPROVAL: not_granted は履歴説明です。",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "MERGE_APPROVAL"):
            check_review_agent_gate.validate_gate(section)

    def test_rejects_duplicate_merge_approval_lines(self):
        packet = VALID_PACKET.replace(
            "MERGE_APPROVAL: not_granted",
            "MERGE_APPROVAL: granted\nMERGE_APPROVAL: not_granted",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "MERGE_APPROVAL"):
            check_review_agent_gate.validate_gate(section)

    def test_rejects_merge_approval_line_with_trailing_comment(self):
        packet = VALID_PACKET.replace(
            "MERGE_APPROVAL: not_granted",
            "MERGE_APPROVAL: granted  # actual\nMERGE_APPROVAL: not_granted",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "MERGE_APPROVAL"):
            check_review_agent_gate.validate_gate(section)

    def test_rejects_na_evidence_for_mandatory_reviewer(self):
        packet = VALID_PACKET.replace(
            "| security_qa | REVIEW_AGENT_OK | security evidence | security reviewed |",
            "| security_qa | REVIEW_AGENT_OK | N/A | security reviewed |",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "security_qa"):
            check_review_agent_gate.validate_gate(section)

    def test_rejects_punctuated_placeholder_evidence_for_mandatory_reviewer(self):
        packet = VALID_PACKET.replace(
            "| functional_qa | REVIEW_AGENT_OK | functional evidence | AC reviewed |",
            "| functional_qa | REVIEW_AGENT_OK | n.a. | AC reviewed |",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "functional_qa"):
            check_review_agent_gate.validate_gate(section)

    def test_rejects_none_evidence_for_forge_gate_equivalent(self):
        packet = VALID_PACKET.replace(
            "| qax2 | REVIEW_AGENT_OK | qax2 evidence | ATO gate reviewed |",
            "| qax2 | REVIEW_AGENT_OK | None | ATO gate reviewed |",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "forge_reviewer"):
            check_review_agent_gate.validate_gate(section)

    def test_rejects_punctuated_placeholder_evidence_for_forge_gate_equivalent(self):
        packet = VALID_PACKET.replace(
            "| qax2 | REVIEW_AGENT_OK | qax2 evidence | ATO gate reviewed |",
            "| qax2 | REVIEW_AGENT_OK | None. | ATO gate reviewed |",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "forge_reviewer"):
            check_review_agent_gate.validate_gate(section)

    def test_rejects_punctuated_placeholder_rationale_for_design_not_applicable(self):
        packet = VALID_PACKET.replace(
            "| design_qa | not_applicable | - | UI / visual 変更なし |",
            "| design_qa | not_applicable | - | N/A. |",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "design_qa"):
            check_review_agent_gate.validate_gate(section)

    def test_accepts_forge_not_applicable_with_risk_tier_low_line(self):
        packet = VALID_PACKET.replace(
            "| qax2 | REVIEW_AGENT_OK | qax2 evidence | ATO gate reviewed |",
            "| forge_reviewer | not_applicable | - | risk_tier=low: docs のみの変更 |",
        ).replace(
            "MERGE_APPROVAL: not_granted",
            "MERGE_APPROVAL: not_granted\n\n"
            "RISK_TIER: low — 全 changed files が delivery_policy.low_risk_paths (docs/**) に一致",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        check_review_agent_gate.validate_gate(section)

    def test_rejects_forge_not_applicable_without_risk_tier_low_line(self):
        packet = VALID_PACKET.replace(
            "| qax2 | REVIEW_AGENT_OK | qax2 evidence | ATO gate reviewed |",
            "| forge_reviewer | not_applicable | - | risk_tier=low: docs のみの変更 |",
        )
        section = check_review_agent_gate.extract_gate_section(packet)
        with self.assertRaisesRegex(check_review_agent_gate.GateError, "RISK_TIER"):
            check_review_agent_gate.validate_gate(section)

    def test_cli_uses_pr_number_packet_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            packet_dir = root / "artifacts/review_packets"
            packet_dir.mkdir(parents=True)
            (packet_dir / "pr-123.md").write_text(VALID_PACKET, encoding="utf-8")
            cwd = Path.cwd()
            try:
                os.chdir(root)
                with patch("sys.argv", ["check_review_agent_gate.py", "--pr-number", "123"]):
                    self.assertEqual(0, check_review_agent_gate.main())
            finally:
                os.chdir(cwd)


if __name__ == "__main__":
    unittest.main()
