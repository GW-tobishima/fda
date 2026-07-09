use std::path::PathBuf;

pub(crate) use crate::application::ports::AtoConfig;
pub(crate) use crate::application::ui::UiConfig;
use crate::domain::entities::CodexLiveStatus;
use crate::domain::value_objects::IntakeMode;
use crate::{DEFAULT_ARTIFACT_DIR, DEFAULT_MODEL_CONTRACT_DIRS, DEFAULT_SCHEMA_DIR};

pub(crate) enum Command {
    Help,
    Start(StartConfig),
    Decide(DecideConfig),
    Design(DesignConfig),
    Plan(PlanConfig),
    Implement(ImplementConfig),
    Review(ReviewConfig),
    Continue(ContinueConfig),
    Merge(MergeConfig),
    Open(OpenConfig),
    Status(StatusConfig),
    NotifyTest(NotifyConfig),
    Ui(UiConfig),
    Gc(GcConfig),
    Policy(PolicyConfig),
    ValidateArtifacts(ValidateConfig),
}

pub(crate) const MAX_LIVE_TIMEOUT_SECONDS: u64 = 86_400;

pub(crate) struct StartConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) input: StartInput,
    pub(crate) out: Option<PathBuf>,
    pub(crate) mode: IntakeMode,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
}

pub(crate) enum StartInput {
    Goal(String),
    File(PathBuf),
}

pub(crate) struct DecideConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifact_dir: PathBuf,
    pub(crate) decision_id: String,
    /// 人間が明示回答する場合の答え。`--by-contract` 使用時は空で、契約の answer を使う。
    pub(crate) answer: String,
    pub(crate) decided_by: String,
    /// `--by-contract <rule_id>`。指定時は delegation contract の answer で回答を記録する。
    /// `--answer` とは排他。
    pub(crate) by_contract: Option<String>,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
}

pub(crate) struct DesignConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifact_dir: PathBuf,
    pub(crate) out: Option<PathBuf>,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
}

pub(crate) struct PlanConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) requirements: PathBuf,
    pub(crate) out: PathBuf,
    pub(crate) mode: PlanMode,
    pub(crate) fixture_dir: PathBuf,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanMode {
    Fixture,
    Model,
}

pub(crate) struct ImplementConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifact_dir: PathBuf,
    pub(crate) out: Option<PathBuf>,
    pub(crate) target_repo: PathBuf,
    pub(crate) dry_run: bool,
    pub(crate) live: bool,
    pub(crate) live_timeout_seconds: u64,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
    pub(crate) tools_list_fixture: Option<Vec<String>>,
    pub(crate) codex_live_fixture: Option<CodexLiveFixture>,
}

pub(crate) struct ReviewConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifact_dir: PathBuf,
    pub(crate) out: Option<PathBuf>,
    pub(crate) target_repo: PathBuf,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
    pub(crate) functional_qa_fixture: Option<QaFixture>,
    pub(crate) security_qa_fixture: Option<QaFixture>,
}

pub(crate) struct ContinueConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifact_dir: PathBuf,
    pub(crate) out: Option<PathBuf>,
    pub(crate) target_repo: PathBuf,
    pub(crate) max_retries: u32,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
    /// `--epic` 指定時は repair gate ではなく F2 Epic 継続ループ（epic.rs）を実行する。
    pub(crate) epic: bool,
    /// `--epic` 時に全 run を走査する root（既定 `artifacts/runs`）。repair path では未使用。
    pub(crate) artifacts_root: PathBuf,
}

pub(crate) struct MergeConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifact_dir: PathBuf,
    pub(crate) out: Option<PathBuf>,
    pub(crate) target_repo: PathBuf,
    pub(crate) execute: bool,
    pub(crate) merge_method: MergeMethod,
    pub(crate) github_merge_command: Option<Vec<String>>,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

impl MergeMethod {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        }
    }

    pub(crate) fn gh_flag(self) -> &'static str {
        match self {
            MergeMethod::Merge => "--merge",
            MergeMethod::Squash => "--squash",
            MergeMethod::Rebase => "--rebase",
        }
    }
}

fn parse_merge_method(value: &str) -> Result<MergeMethod, String> {
    match value {
        "merge" => Ok(MergeMethod::Merge),
        "squash" => Ok(MergeMethod::Squash),
        "rebase" => Ok(MergeMethod::Rebase),
        other => Err(format!(
            "unsupported merge method `{other}`; expected merge, squash, or rebase"
        )),
    }
}

pub(crate) struct OpenConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifact_dir: PathBuf,
    pub(crate) out: Option<PathBuf>,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
}

pub(crate) struct StatusConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifact_dir: PathBuf,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
}

/// `fda gc`（F5 庭師）の設定。read-only スキャン + docket 出力のみで、
/// 既存 run への変更・削除は一切しない。ATO 同期も行わない。
pub(crate) struct GcConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifacts_root: PathBuf,
    pub(crate) max_age_days: u64,
    pub(crate) print_json: bool,
}

/// `fda policy propose`（F1 判断の立法化）の設定。read-only スキャン + 提案出力のみで、
/// `.fda` へは一切書き込まない。ATO 同期も行わない。制定は人間の YAML 編集のみ。
pub(crate) struct PolicyConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifacts_root: PathBuf,
    pub(crate) out: PathBuf,
    pub(crate) min_occurrences: u64,
    pub(crate) print_json: bool,
}

pub(crate) struct NotifyConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) artifact_dir: PathBuf,
    pub(crate) out: Option<PathBuf>,
    pub(crate) channel: String,
    pub(crate) recipient: Option<String>,
    pub(crate) live: bool,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
}

#[derive(Clone)]
pub(crate) struct QaFixture {
    pub(crate) status: String,
    pub(crate) findings: Vec<String>,
    pub(crate) severity: Option<String>,
}

#[derive(Clone)]
pub(crate) struct CodexLiveFixture {
    pub(crate) thread_id: Option<String>,
    pub(crate) content: String,
    pub(crate) status: CodexLiveStatus,
}

pub(crate) struct ValidateConfig {
    pub(crate) repo_root: PathBuf,
    pub(crate) schema_dir: PathBuf,
    pub(crate) artifact_dir: PathBuf,
    pub(crate) out: Option<PathBuf>,
    pub(crate) ato: AtoConfig,
    pub(crate) print_json: bool,
    pub(crate) model_contract_dirs: Vec<PathBuf>,
}

pub(crate) fn parse_args(args: Vec<String>) -> Result<Command, String> {
    if args.is_empty() || args.iter().any(|arg| arg == "-h" || arg == "--help") {
        crate::cli::output::print_help();
        return Ok(Command::Help);
    }

    let command = &args[0];
    if command == "start" {
        return parse_start_args(&args[1..]);
    }
    if command == "decide" {
        return parse_decide_args(&args[1..]);
    }
    if command == "design" {
        return parse_design_args(&args[1..]);
    }
    if command == "plan" {
        return parse_plan_args(&args[1..]);
    }
    if command == "implement" {
        return parse_implement_args(&args[1..]);
    }
    if command == "review" {
        return parse_review_args(&args[1..]);
    }
    if command == "continue" {
        return parse_continue_args(&args[1..]);
    }
    if command == "merge" {
        return parse_merge_args(&args[1..]);
    }
    if command == "open" {
        return parse_open_args(&args[1..]);
    }
    if command == "status" {
        return parse_status_args(&args[1..]);
    }
    if command == "notify" {
        return parse_notify_args(&args[1..]);
    }
    if command == "ui" {
        return parse_ui_args(&args[1..]);
    }
    if command == "gc" {
        return parse_gc_args(&args[1..]);
    }
    if command == "policy" {
        return parse_policy_args(&args[1..]);
    }
    if command != "validate-artifacts" {
        crate::cli::output::print_help();
        return Err(format!("unknown command `{command}`"));
    }

    let mut repo_root = PathBuf::from(".");
    let mut schema_dir = PathBuf::from(DEFAULT_SCHEMA_DIR);
    let mut artifact_dir = PathBuf::from(DEFAULT_ARTIFACT_DIR);
    let mut out = None;
    let mut ato = AtoConfig::default();
    let mut print_json = false;
    let mut model_contract_dirs: Vec<PathBuf> = DEFAULT_MODEL_CONTRACT_DIRS
        .iter()
        .map(PathBuf::from)
        .collect();

    let mut index = 1;
    while index < args.len() {
        if parse_ato_option(&args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(&args, index, "--repo-root")?);
            }
            "--schemas" => {
                index += 1;
                schema_dir = PathBuf::from(expect_value(&args, index, "--schemas")?);
            }
            "--artifacts" => {
                index += 1;
                artifact_dir = PathBuf::from(expect_value(&args, index, "--artifacts")?);
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(expect_value(&args, index, "--out")?));
            }
            "--model-contracts" => {
                index += 1;
                model_contract_dirs.push(PathBuf::from(expect_value(
                    &args,
                    index,
                    "--model-contracts",
                )?));
            }
            "--json" => {
                print_json = true;
            }
            other => {
                return Err(format!("unknown option `{other}`"));
            }
        }
        index += 1;
    }

    Ok(Command::ValidateArtifacts(ValidateConfig {
        repo_root,
        schema_dir,
        artifact_dir,
        out,
        ato,
        print_json,
        model_contract_dirs,
    }))
}

fn expect_value(args: &[String], index: usize, option: &str) -> Result<String, String> {
    args.get(index)
        .cloned()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_ato_option(
    args: &[String],
    index: &mut usize,
    ato: &mut AtoConfig,
) -> Result<bool, String> {
    match args[*index].as_str() {
        "--ato-sync" => {
            ato.enabled = true;
            Ok(true)
        }
        "--ato-task" => {
            *index += 1;
            ato.task_key = Some(expect_value(args, *index, "--ato-task")?);
            Ok(true)
        }
        "--ato-run-id" => {
            *index += 1;
            ato.run_id = Some(expect_value(args, *index, "--ato-run-id")?);
            Ok(true)
        }
        "--ato-backend" => {
            *index += 1;
            ato.backend = Some(expect_value(args, *index, "--ato-backend")?);
            Ok(true)
        }
        "--ato-db" => {
            *index += 1;
            ato.db_path = Some(PathBuf::from(expect_value(args, *index, "--ato-db")?));
            Ok(true)
        }
        "--ato-cli" => {
            *index += 1;
            ato.cli_command = vec![expect_value(args, *index, "--ato-cli")?];
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parse_start_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut input_path = None;
    let mut goal_parts = Vec::new();
    let mut out = None;
    let mut mode = IntakeMode::Auto;
    let mut ato = AtoConfig::default();
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--input" => {
                index += 1;
                input_path = Some(PathBuf::from(expect_value(args, index, "--input")?));
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(expect_value(args, index, "--out")?));
            }
            "--mode" => {
                index += 1;
                mode = parse_intake_mode(&expect_value(args, index, "--mode")?)?;
            }
            "--json" => {
                print_json = true;
            }
            other if other.starts_with('-') => return Err(format!("unknown option `{other}`")),
            other => goal_parts.push(other.to_string()),
        }
        index += 1;
    }

    let goal = goal_parts.join(" ");
    let input = match (goal.is_empty(), input_path) {
        (false, None) => StartInput::Goal(goal),
        (true, Some(path)) => StartInput::File(path),
        (true, None) => return Err("start requires a goal or --input <path>".to_string()),
        (false, Some(_)) => {
            return Err("start accepts either a goal or --input, not both".to_string())
        }
    };

    Ok(Command::Start(StartConfig {
        repo_root,
        input,
        out,
        mode,
        ato,
        print_json,
    }))
}

fn parse_intake_mode(value: &str) -> Result<IntakeMode, String> {
    match value {
        "auto" => Ok(IntakeMode::Auto),
        "implement" => Ok(IntakeMode::Implement),
        "research" => Ok(IntakeMode::Research),
        "uiux" => Ok(IntakeMode::Uiux),
        "design-only" => Ok(IntakeMode::DesignOnly),
        other => Err(format!("unsupported intake mode `{other}`")),
    }
}

fn parse_decide_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut artifact_dir = PathBuf::from(".");
    let mut decision_id = None;
    let mut answer = None;
    let mut by_contract = None;
    let mut decided_by = "human".to_string();
    let mut ato = AtoConfig::default();
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts" => {
                index += 1;
                artifact_dir = PathBuf::from(expect_value(args, index, "--artifacts")?);
            }
            "--answer" => {
                index += 1;
                answer = Some(expect_value(args, index, "--answer")?);
            }
            "--by-contract" => {
                index += 1;
                by_contract = Some(expect_value(args, index, "--by-contract")?);
            }
            "--decided-by" => {
                index += 1;
                decided_by = expect_value(args, index, "--decided-by")?;
            }
            "--json" => {
                print_json = true;
            }
            other if other.starts_with('-') => return Err(format!("unknown option `{other}`")),
            other => {
                if decision_id.is_some() {
                    return Err(format!("unexpected positional argument `{other}`"));
                }
                decision_id = Some(other.to_string());
            }
        }
        index += 1;
    }

    if answer.is_some() && by_contract.is_some() {
        return Err("--answer and --by-contract are mutually exclusive".to_string());
    }
    if answer.is_none() && by_contract.is_none() {
        return Err("decide requires --answer <answer> or --by-contract <rule-id>".to_string());
    }

    Ok(Command::Decide(DecideConfig {
        repo_root,
        artifact_dir,
        decision_id: decision_id.ok_or_else(|| "decide requires <decision-id>".to_string())?,
        answer: answer.unwrap_or_default(),
        decided_by,
        by_contract,
        ato,
        print_json,
    }))
}

fn parse_design_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut artifact_dir = PathBuf::from(".");
    let mut out = None;
    let mut ato = AtoConfig::default();
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts" => {
                index += 1;
                artifact_dir = PathBuf::from(expect_value(args, index, "--artifacts")?);
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(expect_value(args, index, "--out")?));
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::Design(DesignConfig {
        repo_root,
        artifact_dir,
        out,
        ato,
        print_json,
    }))
}

fn parse_plan_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut requirements = None;
    let mut out = None;
    let mut mode = PlanMode::Fixture;
    let mut fixture_dir = PathBuf::from(DEFAULT_ARTIFACT_DIR);
    let mut ato = AtoConfig::default();
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--requirements" => {
                index += 1;
                requirements = Some(PathBuf::from(expect_value(args, index, "--requirements")?));
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(expect_value(args, index, "--out")?));
            }
            "--mode" => {
                index += 1;
                mode = parse_plan_mode(&expect_value(args, index, "--mode")?)?;
            }
            "--fixture-dir" => {
                index += 1;
                fixture_dir = PathBuf::from(expect_value(args, index, "--fixture-dir")?);
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::Plan(PlanConfig {
        repo_root,
        requirements: requirements.ok_or_else(|| "--requirements is required".to_string())?,
        out: out.ok_or_else(|| "--out is required".to_string())?,
        mode,
        fixture_dir,
        ato,
        print_json,
    }))
}

fn parse_plan_mode(value: &str) -> Result<PlanMode, String> {
    match value {
        "fixture" => Ok(PlanMode::Fixture),
        "model" => Ok(PlanMode::Model),
        other => Err(format!("unsupported plan mode `{other}`")),
    }
}

fn parse_implement_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut artifact_dir = PathBuf::from(".");
    let mut out = None;
    let mut target_repo = PathBuf::from(".");
    let mut dry_run = false;
    let mut live = false;
    let mut live_timeout_seconds = 1800;
    let mut ato = AtoConfig::default();
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts" => {
                index += 1;
                artifact_dir = PathBuf::from(expect_value(args, index, "--artifacts")?);
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(expect_value(args, index, "--out")?));
            }
            "--target-repo" => {
                index += 1;
                target_repo = PathBuf::from(expect_value(args, index, "--target-repo")?);
            }
            "--dry-run" => {
                dry_run = true;
            }
            "--live" => {
                live = true;
            }
            "--live-timeout-seconds" => {
                index += 1;
                live_timeout_seconds = expect_value(args, index, "--live-timeout-seconds")?
                    .parse::<u64>()
                    .map_err(|_| {
                        "--live-timeout-seconds requires a positive integer".to_string()
                    })?;
                if live_timeout_seconds == 0 {
                    return Err("--live-timeout-seconds requires a positive integer".to_string());
                }
                if live_timeout_seconds > MAX_LIVE_TIMEOUT_SECONDS {
                    return Err(format!(
                        "--live-timeout-seconds must be <= {MAX_LIVE_TIMEOUT_SECONDS}"
                    ));
                }
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    if dry_run == live {
        return Err("implement requires exactly one of --dry-run or --live".to_string());
    }

    Ok(Command::Implement(ImplementConfig {
        repo_root,
        artifact_dir,
        out,
        target_repo,
        dry_run,
        live,
        live_timeout_seconds,
        ato,
        print_json,
        tools_list_fixture: None,
        codex_live_fixture: None,
    }))
}

fn parse_review_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut artifact_dir = PathBuf::from(".");
    let mut out = None;
    let mut target_repo = PathBuf::from(".");
    let mut ato = AtoConfig::default();
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts" => {
                index += 1;
                artifact_dir = PathBuf::from(expect_value(args, index, "--artifacts")?);
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(expect_value(args, index, "--out")?));
            }
            "--target-repo" => {
                index += 1;
                target_repo = PathBuf::from(expect_value(args, index, "--target-repo")?);
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::Review(ReviewConfig {
        repo_root,
        artifact_dir,
        out,
        target_repo,
        ato,
        print_json,
        functional_qa_fixture: None,
        security_qa_fixture: None,
    }))
}

fn parse_continue_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut artifact_dir = PathBuf::from(".");
    let mut out = None;
    let mut target_repo = PathBuf::from(".");
    let mut max_retries = 3;
    let mut ato = AtoConfig::default();
    let mut print_json = false;
    let mut epic = false;
    let mut artifacts_root = PathBuf::from("artifacts/runs");

    let mut index = 0;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts" => {
                index += 1;
                artifact_dir = PathBuf::from(expect_value(args, index, "--artifacts")?);
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(expect_value(args, index, "--out")?));
            }
            "--target-repo" => {
                index += 1;
                target_repo = PathBuf::from(expect_value(args, index, "--target-repo")?);
            }
            "--max-retries" => {
                index += 1;
                max_retries = expect_value(args, index, "--max-retries")?
                    .parse::<u32>()
                    .map_err(|_| "--max-retries requires a non-negative integer".to_string())?;
            }
            "--epic" => {
                epic = true;
            }
            "--artifacts-root" => {
                index += 1;
                artifacts_root = PathBuf::from(expect_value(args, index, "--artifacts-root")?);
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::Continue(ContinueConfig {
        repo_root,
        artifact_dir,
        out,
        target_repo,
        max_retries,
        ato,
        print_json,
        epic,
        artifacts_root,
    }))
}

fn parse_merge_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut artifact_dir = PathBuf::from(".");
    let mut out = None;
    let mut target_repo = PathBuf::from(".");
    let mut execute = false;
    let mut merge_method = MergeMethod::Merge;
    let mut ato = AtoConfig::default();
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts" => {
                index += 1;
                artifact_dir = PathBuf::from(expect_value(args, index, "--artifacts")?);
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(expect_value(args, index, "--out")?));
            }
            "--target-repo" => {
                index += 1;
                target_repo = PathBuf::from(expect_value(args, index, "--target-repo")?);
            }
            "--execute" => {
                execute = true;
            }
            "--merge-method" => {
                index += 1;
                let value = expect_value(args, index, "--merge-method")?;
                merge_method = parse_merge_method(&value)?;
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::Merge(MergeConfig {
        repo_root,
        artifact_dir,
        out,
        target_repo,
        execute,
        merge_method,
        github_merge_command: None,
        ato,
        print_json,
    }))
}

fn parse_open_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut artifact_dir = PathBuf::from(".");
    let mut out = None;
    let mut ato = AtoConfig::default();
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts" => {
                index += 1;
                artifact_dir = PathBuf::from(expect_value(args, index, "--artifacts")?);
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(expect_value(args, index, "--out")?));
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::Open(OpenConfig {
        repo_root,
        artifact_dir,
        out,
        ato,
        print_json,
    }))
}

fn parse_status_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut artifact_dir = PathBuf::from(".");
    let mut ato = AtoConfig::default();
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts" => {
                index += 1;
                artifact_dir = PathBuf::from(expect_value(args, index, "--artifacts")?);
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::Status(StatusConfig {
        repo_root,
        artifact_dir,
        ato,
        print_json,
    }))
}

fn parse_ui_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut runs_root = PathBuf::from("artifacts/runs");
    let mut port: u16 = 4870;
    let mut open_browser = false;
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts-root" => {
                index += 1;
                runs_root = PathBuf::from(expect_value(args, index, "--artifacts-root")?);
            }
            "--port" => {
                index += 1;
                port = expect_value(args, index, "--port")?
                    .parse()
                    .map_err(|e| format!("--port must be a number: {e}"))?;
            }
            "--open" => {
                open_browser = true;
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::Ui(UiConfig {
        repo_root,
        runs_root,
        port,
        open_browser,
        print_json,
    }))
}

fn parse_gc_args(args: &[String]) -> Result<Command, String> {
    let mut repo_root = PathBuf::from(".");
    let mut artifacts_root = PathBuf::from("artifacts/runs");
    let mut max_age_days: u64 = 30;
    let mut print_json = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts-root" => {
                index += 1;
                artifacts_root = PathBuf::from(expect_value(args, index, "--artifacts-root")?);
            }
            "--max-age-days" => {
                index += 1;
                max_age_days = expect_value(args, index, "--max-age-days")?
                    .parse::<u64>()
                    .map_err(|_| "--max-age-days requires a non-negative integer".to_string())?;
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::Gc(GcConfig {
        repo_root,
        artifacts_root,
        max_age_days,
        print_json,
    }))
}

fn parse_policy_args(args: &[String]) -> Result<Command, String> {
    if args.first().map(String::as_str) != Some("propose") {
        return Err("policy requires subcommand `propose`".to_string());
    }
    let mut repo_root = PathBuf::from(".");
    let mut artifacts_root = PathBuf::from("artifacts/runs");
    let mut out = PathBuf::from("artifacts/runs/_policy");
    let mut min_occurrences: u64 = 3;
    let mut print_json = false;

    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts-root" => {
                index += 1;
                artifacts_root = PathBuf::from(expect_value(args, index, "--artifacts-root")?);
            }
            "--out" => {
                index += 1;
                out = PathBuf::from(expect_value(args, index, "--out")?);
            }
            "--min-occurrences" => {
                index += 1;
                min_occurrences = expect_value(args, index, "--min-occurrences")?
                    .parse::<u64>()
                    .map_err(|_| "--min-occurrences requires a positive integer".to_string())?;
                if min_occurrences == 0 {
                    return Err("--min-occurrences requires a positive integer".to_string());
                }
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::Policy(PolicyConfig {
        repo_root,
        artifacts_root,
        out,
        min_occurrences,
        print_json,
    }))
}

fn parse_notify_args(args: &[String]) -> Result<Command, String> {
    if args.first().map(String::as_str) != Some("test") {
        return Err("notify requires subcommand `test`".to_string());
    }
    let mut repo_root = PathBuf::from(".");
    let mut artifact_dir = PathBuf::from(".");
    let mut out = None;
    let mut channel = "slack".to_string();
    let mut recipient = None;
    let mut live = false;
    let mut ato = AtoConfig::default();
    let mut print_json = false;

    let mut index = 1;
    while index < args.len() {
        if parse_ato_option(args, &mut index, &mut ato)? {
            index += 1;
            continue;
        }
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = PathBuf::from(expect_value(args, index, "--repo-root")?);
            }
            "--artifacts" => {
                index += 1;
                artifact_dir = PathBuf::from(expect_value(args, index, "--artifacts")?);
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(expect_value(args, index, "--out")?));
            }
            "--channel" => {
                index += 1;
                channel = expect_value(args, index, "--channel")?;
            }
            "--to" => {
                index += 1;
                recipient = Some(expect_value(args, index, "--to")?);
            }
            "--live" => {
                live = true;
            }
            "--json" => {
                print_json = true;
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(Command::NotifyTest(NotifyConfig {
        repo_root,
        artifact_dir,
        out,
        channel,
        recipient,
        live,
        ato,
        print_json,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn decide_with_answer_parses() {
        match parse_args(args(&["decide", "HD-FDA-001", "--answer", "yes"])) {
            Ok(Command::Decide(config)) => {
                assert_eq!(config.answer, "yes");
                assert!(config.by_contract.is_none());
            }
            _ => panic!("expected decide command"),
        }
    }

    #[test]
    fn decide_with_by_contract_parses() {
        match parse_args(args(&["decide", "HD-FDA-001", "--by-contract", "DC-001"])) {
            Ok(Command::Decide(config)) => {
                assert_eq!(config.by_contract.as_deref(), Some("DC-001"));
                assert_eq!(config.answer, "");
            }
            _ => panic!("expected decide command"),
        }
    }

    #[test]
    fn decide_answer_and_by_contract_are_mutually_exclusive() {
        match parse_args(args(&[
            "decide",
            "HD-FDA-001",
            "--answer",
            "yes",
            "--by-contract",
            "DC-001",
        ])) {
            Err(err) => assert!(err.contains("mutually exclusive")),
            Ok(_) => panic!("expected mutual-exclusion error"),
        }
    }

    #[test]
    fn decide_requires_answer_or_by_contract() {
        match parse_args(args(&["decide", "HD-FDA-001"])) {
            Err(err) => assert!(err.contains("--answer") && err.contains("--by-contract")),
            Ok(_) => panic!("expected required-one error"),
        }
    }

    #[test]
    fn continue_without_epic_defaults_to_repair() {
        match parse_args(args(&["continue"])) {
            Ok(Command::Continue(config)) => {
                assert!(!config.epic);
                assert_eq!(config.artifacts_root, PathBuf::from("artifacts/runs"));
            }
            _ => panic!("expected continue command"),
        }
    }

    #[test]
    fn continue_epic_flag_and_artifacts_root_parse() {
        match parse_args(args(&[
            "continue",
            "--epic",
            "--artifacts",
            "artifacts/runs/epic-run",
            "--artifacts-root",
            "custom/runs",
        ])) {
            Ok(Command::Continue(config)) => {
                assert!(config.epic);
                assert_eq!(
                    config.artifact_dir,
                    PathBuf::from("artifacts/runs/epic-run")
                );
                assert_eq!(config.artifacts_root, PathBuf::from("custom/runs"));
            }
            _ => panic!("expected continue command"),
        }
    }

    #[test]
    fn policy_propose_defaults_parse() {
        match parse_args(args(&["policy", "propose"])) {
            Ok(Command::Policy(config)) => {
                assert_eq!(config.min_occurrences, 3);
                assert_eq!(config.artifacts_root, PathBuf::from("artifacts/runs"));
                assert_eq!(config.out, PathBuf::from("artifacts/runs/_policy"));
            }
            _ => panic!("expected policy command"),
        }
    }

    #[test]
    fn policy_requires_propose_subcommand() {
        match parse_args(args(&["policy"])) {
            Err(err) => assert!(err.contains("propose")),
            Ok(_) => panic!("expected propose subcommand error"),
        }
    }
}
