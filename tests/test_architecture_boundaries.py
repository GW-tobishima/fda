import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "scripts/check_architecture_boundaries.py"

spec = importlib.util.spec_from_file_location("check_architecture_boundaries", MODULE_PATH)
check_architecture_boundaries = importlib.util.module_from_spec(spec)
assert spec and spec.loader
sys.modules[spec.name] = check_architecture_boundaries
spec.loader.exec_module(check_architecture_boundaries)


FACADE_BODY = """\
pub(crate) fn implement(config: &ImplementConfig) -> Result<ImplementResult, String> {
    application::implement::implement(config, &CodexMcpProcessAdapter)
}

pub(crate) fn review(config: &ReviewConfig) -> Result<ReviewResult, String> {
    application::review::review(config)
}

pub(crate) fn continue_run(config: &ContinueConfig) -> Result<ContinueResult, String> {
    application::repair::continue_run(config)
}

pub(crate) fn merge_run(config: &MergeConfig) -> Result<MergeResult, String> {
    application::merge::merge_run(config)
}

pub(crate) fn open_output_hub(config: &OpenConfig) -> Result<OpenResult, String> {
    application::output_hub::open_output_hub(config)
}

pub(crate) fn notify_test(config: &NotifyConfig) -> Result<NotifyResult, String> {
    application::notify::notify_test(config)
}
"""


class ArchitectureBoundaryGateTest(unittest.TestCase):
    def run_lib_gate(self, lib_body: str, max_lines: int = 520) -> list[str]:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            src = root / "src"
            src.mkdir()
            (src / "lib.rs").write_text(lib_body, encoding="utf-8")
            with patch.object(check_architecture_boundaries, "REPO_ROOT", root), patch.object(
                check_architecture_boundaries, "SRC_DIR", src
            ), patch.object(
                check_architecture_boundaries, "LIB_MAX_NON_TEST_LINES", max_lines
            ):
                return check_architecture_boundaries.check_lib_facade()

    def test_accepts_required_direct_facades(self):
        self.assertEqual([], self.run_lib_gate(FACADE_BODY))

    def test_rejects_non_direct_facade_body(self):
        body = FACADE_BODY.replace(
            "application::review::review(config)",
            "let result = application::review::review(config)?;\n    Ok(result)",
        )
        failures = self.run_lib_gate(body)
        self.assertTrue(any("facade `review` must delegate directly" in failure for failure in failures))

    def test_rejects_helper_name_before_required_facade(self):
        body = (
            "pub(crate) fn review_helper(config: &ReviewConfig) -> Result<ReviewResult, String> {\n"
            "    application::review::review(config)\n"
            "}\n\n"
            + FACADE_BODY.replace(
                "application::review::review(config)",
                "let result = application::review::review(config)?;\n    Ok(result)",
            )
        )
        failures = self.run_lib_gate(body)
        self.assertTrue(any("facade `review` must delegate directly" in failure for failure in failures))

    def test_rejects_lib_process_adapter_helper_import(self):
        body = "use crate::infra::process::run_process_command;\n" + FACADE_BODY
        failures = self.run_lib_gate(body)
        self.assertTrue(any("run_process_command" in failure for failure in failures))

    def test_rejects_non_test_line_growth(self):
        body = "\n".join(["fn helper() {}"] * 5) + "\n" + FACADE_BODY
        failures = self.run_lib_gate(body, max_lines=4)
        self.assertTrue(any("non-test section" in failure for failure in failures))

    def test_ignores_test_section_for_line_count_and_forbidden_process(self):
        body = (
            FACADE_BODY
            + "\n#[cfg(test)]\nmod tests {\n"
            + "\n".join(["fn noisy_test() { let _ = std::process::id(); }"] * 20)
            + "\n}\n"
        )
        self.assertEqual([], self.run_lib_gate(body, max_lines=40))

    def test_rejects_infra_to_cli_dependency(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            infra = root / "src" / "infra"
            infra.mkdir(parents=True)
            (infra / "adapter.rs").write_text(
                "use crate::cli::args::AtoConfig;\n", encoding="utf-8"
            )
            with patch.object(check_architecture_boundaries, "REPO_ROOT", root):
                failures = check_architecture_boundaries.find_forbidden(
                    infra, check_architecture_boundaries.INFRA_FORBIDDEN_SNIPPETS
                )
        self.assertTrue(any("crate::cli" in failure for failure in failures))

    def test_rejects_cli_grouped_std_filesystem_and_process_imports(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cli = root / "src" / "cli"
            cli.mkdir(parents=True)
            (cli / "runner.rs").write_text(
                "use std::{fs, process::Command};\n", encoding="utf-8"
            )
            with patch.object(check_architecture_boundaries, "REPO_ROOT", root), patch.object(
                check_architecture_boundaries, "SRC_DIR", root / "src"
            ):
                failures = check_architecture_boundaries.check_cli_grouped_std_imports()
        self.assertTrue(any("forbidden grouped std import" in failure for failure in failures))

    def test_rejects_cli_infra_alias_imports(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cli = root / "src" / "cli"
            cli.mkdir(parents=True)
            (cli / "runner.rs").write_text(
                "use crate::infra as adapters;\n", encoding="utf-8"
            )
            with patch.object(check_architecture_boundaries, "REPO_ROOT", root), patch.object(
                check_architecture_boundaries, "SRC_DIR", root / "src"
            ):
                failures = check_architecture_boundaries.check_cli_infra_allowlist()
        self.assertTrue(any("undocumented infra alias import" in failure for failure in failures))

    def test_rejects_infra_application_alias_imports(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            infra = root / "src" / "infra"
            infra.mkdir(parents=True)
            (infra / "adapter.rs").write_text(
                "use crate::application as app;\n", encoding="utf-8"
            )
            with patch.object(check_architecture_boundaries, "REPO_ROOT", root), patch.object(
                check_architecture_boundaries, "SRC_DIR", root / "src"
            ):
                failures = check_architecture_boundaries.check_infra_application_allowlist()
        self.assertTrue(
            any("undocumented application alias import" in failure for failure in failures)
        )


if __name__ == "__main__":
    unittest.main()
