#!/usr/bin/env python3
"""Lightweight source architecture gate for fda v1."""

from __future__ import annotations

import re
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SRC_DIR = REPO_ROOT / "src"
LIB_MAX_NON_TEST_LINES = 520

ALLOWED_APPLICATION_INFRA_USES = {
    "src/application/start.rs": {
        "use crate::infra::clock::SystemClock;",
        "use crate::infra::fs_store::FsArtifactStore;",
    },
    "src/application/status.rs": {
        "use crate::infra::clock::system_unix_seconds;",
        "use crate::infra::fs_store::FsArtifactStore;",
        "use crate::infra::yaml::SerdeYamlValidator;",
    },
    "src/application/design.rs": {
        "use crate::infra::clock::SystemClock;",
        "use crate::infra::fs_store::FsArtifactStore;",
    },
    "src/application/decide.rs": {
        "use crate::infra::clock::SystemClock;",
        "use crate::infra::fs_store::FsArtifactStore;",
        "use crate::infra::yaml::SerdeYamlValidator;",
    },
    "src/application/gc.rs": {
        "use crate::infra::clock::system_unix_seconds;",
        "use crate::infra::fs_store::{list_dir_names, modified_unix_seconds, FsArtifactStore};",
    },
    "src/application/policy.rs": {
        "use crate::infra::clock::system_unix_seconds;",
        "use crate::infra::fs_store::{list_dir_names, FsArtifactStore};",
        "use crate::infra::json_schema::JsonSchemaArtifactValidator;",
    },
    "src/application/risk_tier.rs": {
        "use crate::infra::yaml::SerdeYamlValidator;",
        "use crate::infra::clock::system_unix_seconds;",
        "use crate::infra::fs_store::FsArtifactStore;",
    },
    "src/application/plan.rs": {
        "use crate::infra::fs_store::FsArtifactStore;",
    },
    "src/application/profile.rs": {
        "use crate::infra::json_schema::JsonSchemaArtifactValidator;",
        "use crate::infra::yaml::SerdeYamlValidator;",
    },
    "src/application/implement.rs": {
        "use crate::infra::fs_store::FsArtifactStore;",
        "use crate::infra::json_file::{read_json_value, write_json_file};",
        "use crate::infra::paths::{canonicalize_existing, canonicalize_existing_or_parent};",
    },
    "src/application/review.rs": {
        "use crate::infra::fs_store::FsArtifactStore;",
        "use crate::infra::json_file::{read_json_value, write_json_file};",
        "use crate::infra::paths::canonicalize_existing_or_parent;",
    },
    "src/application/repair.rs": {
        "use crate::infra::fs_store::FsArtifactStore;",
        "use crate::infra::json_file::{read_json_value, write_json_file};",
        "use crate::infra::paths::canonicalize_existing_or_parent;",
    },
    "src/application/merge.rs": {
        "use crate::infra::fs_store::FsArtifactStore;",
        "use crate::infra::json_file::{read_json_value, write_json_file};",
        "use crate::infra::json_schema::JsonSchemaArtifactValidator;",
        "use crate::infra::paths::canonicalize_existing_or_parent;",
        "use crate::infra::process::{python_launcher, run_process_command};",
        "use crate::infra::yaml::SerdeYamlValidator;",
    },
    "src/application/notify.rs": {
        "use crate::infra::fs_store::FsArtifactStore;",
        "use crate::infra::git::repo_project_name;",
        "use crate::infra::json_file::write_json_file;",
        "use crate::infra::slack::{",
        "use crate::infra::smtp::{",
    },
    "src/application/output_hub.rs": {
        "use crate::infra::fs_store::{list_file_names, FsArtifactStore};",
        "use crate::infra::json_file::{read_json_value, write_json_file};",
    },
    "src/application/ui.rs": {
        "use crate::infra::clock::system_unix_seconds;",
        "use crate::infra::fs_store::{list_dir_names, list_file_names, FsArtifactStore};",
    },
    "src/application/validate.rs": {
        "use crate::infra::clock::SystemClock;",
        "use crate::infra::fs_store::FsArtifactStore;",
        "use crate::infra::json_schema::{read_json, JsonSchemaArtifactValidator};",
        "use crate::infra::yaml::{validate_yaml_dir, SerdeYamlValidator};",
    },
}

MAIN_FORBIDDEN_SNIPPETS = [
    "std::fs",
    "std::process::Command",
    "jsonschema",
    "serde_yaml",
    "forge_delivery_agent::application",
    "forge_delivery_agent::domain",
    "forge_delivery_agent::infra",
    "forge_delivery_agent::rendering",
]

LIB_FORBIDDEN_SNIPPETS = [
    "std::process",
    "TcpStream",
    "ToSocketAddrs",
    "Command as ProcessCommand",
    "run_process_command",
    "query_codex_live_tool",
    "jsonschema",
    "serde_yaml",
    "println!",
    "eprintln!",
]

LIB_REQUIRED_FACADES = {
    "implement": "application::implement::implement(config, &CodexMcpProcessAdapter)",
    "review": "application::review::review(config)",
    "continue_run": "application::repair::continue_run(config)",
    "merge_run": "application::merge::merge_run(config)",
    "open_output_hub": "application::output_hub::open_output_hub(config)",
    "notify_test": "application::notify::notify_test(config)",
}

DOMAIN_FORBIDDEN_SNIPPETS = [
    "crate::application",
    "crate::infra",
    "std::fs",
    "std::process",
    "SystemTime",
    "UNIX_EPOCH",
    "jsonschema",
    "serde_yaml",
    "println!",
    "eprintln!",
]

RENDERING_FORBIDDEN_SNIPPETS = [
    "crate::infra",
    "std::fs",
    "std::process",
    "SystemTime",
    "UNIX_EPOCH",
    "jsonschema",
    "serde_yaml",
    "println!",
    "eprintln!",
]

APPLICATION_FORBIDDEN_SNIPPETS = [
    "std::fs",
    "std::process",
    "SystemTime",
    "UNIX_EPOCH",
    "jsonschema",
    "serde_yaml",
    "println!",
    "eprintln!",
]

CLI_FORBIDDEN_SNIPPETS = [
    "std::fs",
    "std::process",
    "SystemTime",
    "UNIX_EPOCH",
    "jsonschema",
    "serde_yaml",
]

CLI_GROUPED_STD_FORBIDDEN_PATTERNS = [
    r"(^|[{\s,])fs($|[,\s}:])",
    r"(^|[{\s,])process($|[,\s}:])",
    r"process::",
    r"SystemTime",
    r"UNIX_EPOCH",
    r"jsonschema",
    r"serde_yaml",
]

INFRA_FORBIDDEN_SNIPPETS = [
    "crate::cli",
]

ALLOWED_CLI_INFRA_USES = {
    "src/cli/runner.rs": {
        "use crate::infra::ato_state::{",
    },
}

ALLOWED_INFRA_APPLICATION_USES = {
    "src/infra/ato_state.rs": {
        "use crate::application::ports::AtoConfig;",
    },
    "src/infra/clock.rs": {
        "use crate::application::ports::Clock;",
    },
    "src/infra/fs_store.rs": {
        "use crate::application::ports::ArtifactStore;",
    },
    "src/infra/json_schema.rs": {
        "use crate::application::ports::{ArtifactStore, ArtifactValidator, CheckError};",
    },
    "src/infra/process.rs": {
        "use crate::application::ports::{CodexProcessPort, ProcessOutput};",
    },
    "src/infra/ui_server.rs": {
        "use crate::application::ui::{mission_control_snapshot, UiConfig};",
    },
    "src/infra/yaml.rs": {
        "use crate::application::ports::{ArtifactStore, CheckError, YamlValidator};",
        "use crate::application::validate::{failed_check, passed_check, skipped_check, ValidationCheck};",
    },
}


def relative(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def find_forbidden(root: Path, forbidden: list[str]) -> list[str]:
    failures: list[str] = []
    for path in sorted(root.rglob("*.rs")):
        body = read_text(path)
        for line_no, line in enumerate(body.splitlines(), start=1):
            for snippet in forbidden:
                if snippet in line:
                    failures.append(f"{relative(path)}:{line_no}: forbidden `{snippet}`")
    return failures


def check_main() -> list[str]:
    failures: list[str] = []
    main = SRC_DIR / "main.rs"
    body = read_text(main)
    line_count = len(body.splitlines())
    if line_count > 100:
        failures.append(f"src/main.rs has {line_count} lines; expected <= 100")

    for line_no, line in enumerate(body.splitlines(), start=1):
        for snippet in MAIN_FORBIDDEN_SNIPPETS:
            if snippet in line:
                failures.append(f"src/main.rs:{line_no}: forbidden `{snippet}`")
    return failures


def lib_non_test_body() -> tuple[str, int]:
    lib = SRC_DIR / "lib.rs"
    lines = read_text(lib).splitlines()
    test_start = next(
        (
            index
            for index, line in enumerate(lines)
            if line.strip() == "mod tests {"
            and index > 0
            and lines[index - 1].strip() == "#[cfg(test)]"
        ),
        len(lines),
    )
    return "\n".join(lines[:test_start]), test_start


def check_lib_facade() -> list[str]:
    failures: list[str] = []
    body, line_count = lib_non_test_body()
    if line_count > LIB_MAX_NON_TEST_LINES:
        failures.append(
            f"src/lib.rs non-test section has {line_count} lines; expected <= {LIB_MAX_NON_TEST_LINES}"
        )

    for line_no, line in enumerate(body.splitlines(), start=1):
        for snippet in LIB_FORBIDDEN_SNIPPETS:
            if snippet in line:
                failures.append(f"src/lib.rs:{line_no}: forbidden `{snippet}`")

    compact = "\n".join(line.rstrip() for line in body.splitlines())
    for function_name, expected_call in LIB_REQUIRED_FACADES.items():
        signature_pattern = re.compile(
            rf"(?m)^pub\(crate\)\s+fn\s+{re.escape(function_name)}\s*"
            r"\([^)]*\)\s*->\s*[^\n{]+\{\s*$"
        )
        matches = list(signature_pattern.finditer(compact))
        if not matches:
            failures.append(f"src/lib.rs missing facade `{function_name}`")
            continue
        if len(matches) > 1:
            failures.append(f"src/lib.rs facade `{function_name}` is duplicated")
            continue
        body_start = compact.find("{", matches[0].start(), matches[0].end())
        body_end = compact.find("\n}", body_start)
        if body_start < 0 or body_end < 0:
            failures.append(f"src/lib.rs facade `{function_name}` could not be parsed")
            continue
        facade_body = compact[body_start + 1 : body_end].strip()
        if facade_body != expected_call:
            failures.append(
                f"src/lib.rs facade `{function_name}` must delegate directly to `{expected_call}`"
            )
    return failures


def check_application_infra_allowlist() -> list[str]:
    failures: list[str] = []
    for path in sorted((SRC_DIR / "application").rglob("*.rs")):
        rel = relative(path)
        allowed_lines = ALLOWED_APPLICATION_INFRA_USES.get(rel, set())
        for line_no, line in enumerate(read_text(path).splitlines(), start=1):
            stripped = line.strip()
            if "crate::infra::" in stripped and stripped not in allowed_lines:
                failures.append(f"{rel}:{line_no}: undocumented infra use `{stripped}`")
    return failures


def check_infra_application_allowlist() -> list[str]:
    failures: list[str] = []
    for path in sorted((SRC_DIR / "infra").rglob("*.rs")):
        rel = relative(path)
        allowed_lines = ALLOWED_INFRA_APPLICATION_USES.get(rel, set())
        for line_no, line in enumerate(read_text(path).splitlines(), start=1):
            stripped = line.strip()
            if re.search(r"\bapplication\s+as\b", stripped):
                failures.append(
                    f"{rel}:{line_no}: undocumented application alias import `{stripped}`"
                )
            if "application::" in stripped and stripped not in allowed_lines:
                failures.append(f"{rel}:{line_no}: undocumented application use `{stripped}`")
    return failures


def check_cli_infra_allowlist() -> list[str]:
    failures: list[str] = []
    for path in sorted((SRC_DIR / "cli").rglob("*.rs")):
        rel = relative(path)
        allowed_lines = ALLOWED_CLI_INFRA_USES.get(rel, set())
        for line_no, line in enumerate(read_text(path).splitlines(), start=1):
            stripped = line.strip()
            if re.search(r"\binfra\s+as\b", stripped):
                failures.append(f"{rel}:{line_no}: undocumented infra alias import `{stripped}`")
            if "infra::" in stripped and stripped not in allowed_lines:
                failures.append(f"{rel}:{line_no}: undocumented infra use `{stripped}`")
    return failures


def check_cli_grouped_std_imports() -> list[str]:
    failures: list[str] = []
    for path in sorted((SRC_DIR / "cli").rglob("*.rs")):
        rel = relative(path)
        grouped_start_line: int | None = None
        grouped_import = ""
        for line_no, line in enumerate(read_text(path).splitlines(), start=1):
            stripped = line.strip()
            if grouped_start_line is None and stripped.startswith("use std::{"):
                grouped_start_line = line_no
                grouped_import = stripped
            elif grouped_start_line is not None:
                grouped_import = f"{grouped_import} {stripped}"

            if grouped_start_line is not None and ";" in stripped:
                for pattern in CLI_GROUPED_STD_FORBIDDEN_PATTERNS:
                    if re.search(pattern, grouped_import):
                        failures.append(
                            f"{rel}:{grouped_start_line}: forbidden grouped std import `{grouped_import}`"
                        )
                        break
                grouped_start_line = None
                grouped_import = ""
    return failures


def run() -> int:
    failures: list[str] = []
    failures.extend(check_main())
    failures.extend(check_lib_facade())
    failures.extend(find_forbidden(SRC_DIR / "cli", CLI_FORBIDDEN_SNIPPETS))
    failures.extend(find_forbidden(SRC_DIR / "domain", DOMAIN_FORBIDDEN_SNIPPETS))
    failures.extend(find_forbidden(SRC_DIR / "rendering", RENDERING_FORBIDDEN_SNIPPETS))
    failures.extend(find_forbidden(SRC_DIR / "application", APPLICATION_FORBIDDEN_SNIPPETS))
    failures.extend(find_forbidden(SRC_DIR / "infra", INFRA_FORBIDDEN_SNIPPETS))
    failures.extend(check_cli_grouped_std_imports())
    failures.extend(check_cli_infra_allowlist())
    failures.extend(check_application_infra_allowlist())
    failures.extend(check_infra_application_allowlist())

    if failures:
        print("architecture boundary check failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1

    print("architecture boundary check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(run())
