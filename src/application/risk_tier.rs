//! F4 比例ゲート: 変更の risk tier を判定する純ロジックと成果物出力。
//!
//! `assess_risk_tier` は scope パス群と delivery_policy（parsed）だけを見る純関数で、
//! low / standard / high を返す。緩和（forge_reviewer / design_qa の auto
//! not_applicable）は `fda review` が review_agent_gate.json の既存契約
//! （status=not_applicable + not_applicable_reason）として記録し、`fda merge` は
//! `proportional_relaxation` による merge 時再検証（live 再計算 + ガバナンス・
//! ハードガード）でその正当性を検証するだけにする（merge が独自にスキップ判定しない）。
//! ゲートの種類は減らさない = fail-closed 維持。

use serde::Serialize;
use serde_json::Value;
use std::path::Path;

use crate::application::ports::{ArtifactStore, YamlValidator};
use crate::infra::yaml::SerdeYamlValidator;
use crate::support::paths::display_path;

pub(crate) const RISK_TIER_SCHEMA_VERSION: &str = "fda.risk_tier.v1";
const DEFAULT_POLICY_SOURCE: &str = "delivery_policy.low_risk_paths+human_required_for";

/// risk_tier.json のシリアライズ表現。
#[derive(Debug, Clone, Serialize)]
pub(crate) struct RiskTier {
    pub(crate) schema_version: &'static str,
    pub(crate) tier: String,
    pub(crate) reasons: Vec<String>,
    pub(crate) matched_low_risk_paths: Vec<String>,
    pub(crate) policy_source: String,
}

impl RiskTier {
    fn new(tier: &str, reasons: Vec<String>, matched_low_risk_paths: Vec<String>) -> Self {
        RiskTier {
            schema_version: RISK_TIER_SCHEMA_VERSION,
            tier: tier.to_string(),
            reasons,
            matched_low_risk_paths,
            policy_source: DEFAULT_POLICY_SOURCE.to_string(),
        }
    }
}

/// scope パス群と delivery_policy から risk tier を判定する純ロジック。
///
/// 判定順:
/// 1. いずれかの scope パスが human_required_for の security/privacy/legal 系
///    キーワードに該当 → `high`（最優先）。
/// 2. scope が空でなく、全パスが low_risk_paths glob に一致 → `low`。
/// 3. それ以外 → `standard`。
pub(crate) fn assess_risk_tier(scope_paths: &[String], policy: &Value) -> RiskTier {
    let low_risk_globs = policy_string_array(policy, "low_risk_paths");
    let human_required = policy_string_array(policy, "human_required_for");
    let sensitive_keywords = sensitive_keywords_from_human_required(&human_required);

    // 1. human_required_for（security/privacy/legal 系）該当 → high
    let mut high_reasons = Vec::new();
    for path in scope_paths {
        let normalized = path.replace('\\', "/").to_ascii_lowercase();
        if let Some(keyword) = sensitive_keywords
            .iter()
            .find(|keyword| normalized.contains(keyword.as_str()))
        {
            high_reasons.push(format!(
                "scope パス `{path}` は human_required_for キーワード `{keyword}` に該当する可能性があります"
            ));
        }
    }
    if !high_reasons.is_empty() {
        return RiskTier::new("high", high_reasons, Vec::new());
    }

    // 2. 全 scope パスが low_risk_paths に一致 → low
    if !scope_paths.is_empty() {
        let mut matched = Vec::new();
        let mut all_low = true;
        for path in scope_paths {
            if low_risk_globs.iter().any(|glob| glob_match(glob, path)) {
                push_unique(&mut matched, path.clone());
            } else {
                all_low = false;
            }
        }
        if all_low {
            return RiskTier::new(
                "low",
                vec![format!(
                    "全 {} scope パスが delivery_policy.low_risk_paths に一致します",
                    scope_paths.len()
                )],
                matched,
            );
        }
    }

    // 3. standard
    let reason = if scope_paths.is_empty() {
        "scope パスが空のため low とは判定できません（standard 扱い）".to_string()
    } else {
        "一部の scope パスが low_risk_paths に一致しないため standard と判定しました".to_string()
    };
    RiskTier::new("standard", vec![reason], Vec::new())
}

/// 比例緩和の判定結果。review は生成に、merge は検証に使う（判定ロジックは単一）。
#[derive(Debug, Clone)]
pub(crate) struct ProportionalRelaxation {
    /// forge_reviewer を not_applicable にしてよいか。
    pub(crate) forge_reviewer: bool,
    /// design_qa を not_applicable にしてよいか。
    pub(crate) design_qa: bool,
    /// 緩和適用時に記録する理由（"risk_tier=low: ..."）。
    pub(crate) reason: Option<String>,
    /// 緩和を却下した理由（mismatch / hard guard 等。補助情報）。
    pub(crate) notes: Vec<String>,
}

impl ProportionalRelaxation {
    fn denied(notes: Vec<String>) -> Self {
        ProportionalRelaxation {
            forge_reviewer: false,
            design_qa: false,
            reason: None,
            notes,
        }
    }
}

/// governance-critical パス（比例ゲートのハードガード対象）。
///
/// これらの変更を含む PR では、low_risk_paths / risk tier の値に**関係なく**
/// forge_reviewer の緩和を適用しない。この規則はコードにハードコードされており、
/// delivery_policy.yaml では上書きできない（段階的統治弱体化の経路を遮断する）。
pub(crate) fn is_governance_critical_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let normalized = normalized.trim_start_matches("./");
    normalized.starts_with(".fda/")
        || normalized.contains("/.fda/")
        || normalized == "scripts/check_review_agent_gate.py"
        || normalized == "scripts/check_architecture_boundaries.py"
        || normalized == "tests/test_review_agent_gate.py"
        || normalized == ".github/workflows/ci.yml"
        || normalized.starts_with("src/application/merge")
        || normalized.starts_with("src/application/review")
        || normalized.starts_with("src/application/risk_tier")
        || normalized.starts_with("src/application/policy")
}

/// merge 時再検証つきの比例緩和判定（TOCTOU / scope-drift 対策）。
///
/// 保存済み tier（risk_tier.json）を無検証で信頼せず、changed_files を
/// `.fda/delivery_policy.yaml` の low_risk_paths で live 再計算し、
/// **stored=low かつ live=low の両方が成立する場合のみ**緩和を許す。
/// さらに governance-critical パスを含む場合は forge_reviewer の緩和を
/// 却下する（ハードガード。design_qa は UI 無関係のため緩和可のまま）。
pub(crate) fn proportional_relaxation(
    store: &impl ArtifactStore,
    repo_root: &Path,
    stored_tier: Option<&str>,
    changed_files: &[String],
) -> ProportionalRelaxation {
    if stored_tier != Some("low") {
        return ProportionalRelaxation::denied(Vec::new());
    }
    let live = match load_delivery_policy(store, repo_root) {
        Ok((policy, _)) => assess_risk_tier(changed_files, &policy),
        Err(error) => {
            return ProportionalRelaxation::denied(vec![format!(
                "delivery_policy の読み込みに失敗したため緩和不適用: {error}"
            )]);
        }
    };
    let mut notes = Vec::new();
    let governance: Vec<&str> = changed_files
        .iter()
        .filter(|path| is_governance_critical_path(path))
        .map(String::as_str)
        .collect();
    if !governance.is_empty() {
        notes.push(format!(
            "governance-critical path を検出したため forge_reviewer は緩和不適用 (hard guard, YAML で上書き不可): {}",
            governance.join(", ")
        ));
    }
    if live.tier != "low" {
        notes.push(format!(
            "stored/live tier mismatch (stored=low, live={}): 緩和不適用 (standard 扱い)",
            live.tier
        ));
        return ProportionalRelaxation::denied(notes);
    }
    let reason = format!(
        "risk_tier=low: 全 changed files が delivery_policy.low_risk_paths に一致 (matched: {})",
        live.matched_low_risk_paths.join(", ")
    );
    if !governance.is_empty() {
        return ProportionalRelaxation {
            forge_reviewer: false,
            design_qa: true,
            reason: Some(reason),
            notes,
        };
    }
    ProportionalRelaxation {
        forge_reviewer: true,
        design_qa: true,
        reason: Some(reason),
        notes,
    }
}

/// artifact_dir の risk_tier.json から tier を読む（無ければ None）。
pub(crate) fn stored_risk_tier(store: &impl ArtifactStore, artifact_dir: &Path) -> Option<String> {
    let path = artifact_dir.join("risk_tier.json");
    if !store.exists(&path) {
        return None;
    }
    let body = store.read_text(&path).ok()?;
    let value: Value = serde_json::from_str(&body).ok()?;
    value
        .get("tier")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// `fda implement --dry-run` で risk_tier.json を out_dir に生成する。
///
/// - policy は repo_root/.fda/delivery_policy.yaml を読む（無ければ standard フォールバック）。
/// - scope パスは artifact_dir/planned_prs.json の expected_files から取得する。
/// - risk_register に high/critical があれば high へ昇格する。
///
/// 返り値は書き出した risk_tier.json の repo_root 相対表示パス。
pub(crate) fn write_risk_tier_artifact(
    store: &impl ArtifactStore,
    repo_root: &Path,
    artifact_dir: &Path,
    out_dir: &Path,
) -> Result<String, String> {
    let (policy, policy_source) = load_delivery_policy(store, repo_root)?;
    let scope_paths = scope_paths_from_planned_prs(store, artifact_dir);
    let mut risk_tier = assess_risk_tier(&scope_paths, &policy);
    risk_tier.policy_source = policy_source;

    if risk_tier.tier != "high" {
        if let Some(reason) = risk_register_high_reason(store, artifact_dir)? {
            risk_tier.tier = "high".to_string();
            risk_tier.reasons.push(reason);
            risk_tier.matched_low_risk_paths.clear();
        }
    }

    let path = out_dir.join("risk_tier.json");
    let value = serde_json::to_value(&risk_tier).map_err(|e| e.to_string())?;
    store.write_json(&path, &value)?;
    Ok(display_path(repo_root, &path))
}

fn load_delivery_policy(
    store: &impl ArtifactStore,
    repo_root: &Path,
) -> Result<(Value, String), String> {
    let policy_path = repo_root.join(".fda").join("delivery_policy.yaml");
    if !store.exists(&policy_path) {
        return Ok((
            Value::Null,
            "<missing:.fda/delivery_policy.yaml>".to_string(),
        ));
    }
    let body = store.read_text(&policy_path)?;
    let value = SerdeYamlValidator
        .parse_yaml_value(&policy_path, &body)
        .map_err(|e| format!("failed to parse delivery_policy.yaml for risk tier: {e}"))?;
    Ok((value, ".fda/delivery_policy.yaml".to_string()))
}

fn scope_paths_from_planned_prs(store: &impl ArtifactStore, artifact_dir: &Path) -> Vec<String> {
    let path = artifact_dir.join("planned_prs.json");
    if !store.exists(&path) {
        return Vec::new();
    }
    let Ok(body) = store.read_text(&path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&body) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for pr in value
        .get("planned_prs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for file in pr
            .get("expected_files")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(file) = file.as_str() {
                push_unique(&mut paths, file.to_string());
            }
        }
    }
    paths
}

fn risk_register_high_reason(
    store: &impl ArtifactStore,
    artifact_dir: &Path,
) -> Result<Option<String>, String> {
    let json_path = artifact_dir.join("risk_register.json");
    let markdown_path = artifact_dir.join("risk_register.md");
    let mut values = Vec::new();
    if store.exists(&json_path) {
        if let Ok(value) = serde_json::from_str::<Value>(&store.read_text(&json_path)?) {
            collect_json_strings(&value, &mut values);
        }
    }
    if store.exists(&markdown_path) {
        values.push(store.read_text(&markdown_path)?);
    }
    let escalates = values.iter().any(|value| {
        let normalized = value.to_lowercase().replace(['-', '_'], " ");
        normalized.contains("critical") || normalized.split_whitespace().any(|word| word == "high")
    });
    Ok(escalates.then(|| {
        "risk_register に high/critical リスクを検出したため high へ昇格しました".to_string()
    }))
}

fn policy_string_array(policy: &Value, key: &str) -> Vec<String> {
    let base = policy.get("delivery_policy").unwrap_or(policy);
    base.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn sensitive_keywords_from_human_required(human_required: &[String]) -> Vec<String> {
    const GROUPS: &[(&str, &[&str])] = &[
        ("security", &["security", "secret", "credential", "auth"]),
        ("privacy", &["privacy", "personal", "pii"]),
        ("legal", &["legal", "license"]),
        ("terms", &["terms"]),
        ("user_data", &["user_data", "userdata", "user-data"]),
    ];
    let mut keywords: Vec<String> = Vec::new();
    for token in human_required {
        let lowered = token.to_ascii_lowercase();
        for (marker, expansions) in GROUPS {
            if lowered.contains(marker) {
                for expansion in *expansions {
                    push_unique(&mut keywords, (*expansion).to_string());
                }
            }
        }
    }
    keywords
}

fn collect_json_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(string) => out.push(string.clone()),
        Value::Array(values) => {
            for value in values {
                collect_json_strings(value, out);
            }
        }
        Value::Object(map) => {
            for value in map.values() {
                collect_json_strings(value, out);
            }
        }
        _ => {}
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

/// `**`（スラッシュ跨ぎ）と `*`（同一セグメント内）と `?` に対応する簡易 glob マッチ。
/// 既存 crate に glob 依存が無いため自前実装する。
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.replace('\\', "/").chars().collect();
    let text: Vec<char> = text.replace('\\', "/").chars().collect();
    glob_match_inner(&pattern, 0, &text, 0)
}

fn glob_match_inner(pattern: &[char], pi: usize, text: &[char], ti: usize) -> bool {
    if pi == pattern.len() {
        return ti == text.len();
    }
    match pattern[pi] {
        '*' => {
            if pattern.get(pi + 1) == Some(&'*') {
                // `**`: スラッシュを含む任意の並びに一致（連続する `*` はまとめる）。
                let mut next = pi + 2;
                while pattern.get(next) == Some(&'*') {
                    next += 1;
                }
                (ti..=text.len()).any(|k| glob_match_inner(pattern, next, text, k))
            } else {
                // `*`: スラッシュを含まない任意の並びに一致。
                let mut k = ti;
                loop {
                    if glob_match_inner(pattern, pi + 1, text, k) {
                        return true;
                    }
                    if k >= text.len() || text[k] == '/' {
                        return false;
                    }
                    k += 1;
                }
            }
        }
        '?' => {
            ti < text.len() && text[ti] != '/' && glob_match_inner(pattern, pi + 1, text, ti + 1)
        }
        expected => {
            ti < text.len()
                && text[ti] == expected
                && glob_match_inner(pattern, pi + 1, text, ti + 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::clock::system_unix_seconds;
    use crate::infra::fs_store::FsArtifactStore;
    use serde_json::json;

    fn fda_policy() -> Value {
        json!({
            "delivery_policy": {
                "low_risk_paths": ["docs/**", "tests/**", ".fda/**", "artifacts/runs/**"],
                "human_required_for": [
                    "scope_change",
                    "privacy_policy_change",
                    "terms_change",
                    "legal_judgment",
                    "security_boundary_change",
                    "user_data_exposure"
                ]
            }
        })
    }

    #[test]
    fn all_docs_scope_is_low() {
        let scope = vec![
            "docs/v1/work_protocol.md".to_string(),
            "docs/standards/delivery-artifacts-v0/schemas/risk_tier.schema.json".to_string(),
        ];
        let result = assess_risk_tier(&scope, &fda_policy());
        assert_eq!(result.tier, "low");
        assert_eq!(result.matched_low_risk_paths.len(), 2);
    }

    #[test]
    fn mixed_source_scope_is_standard() {
        let scope = vec![
            "docs/v1/work_protocol.md".to_string(),
            "src/application/merge.rs".to_string(),
        ];
        let result = assess_risk_tier(&scope, &fda_policy());
        assert_eq!(result.tier, "standard");
        assert!(result.matched_low_risk_paths.is_empty());
    }

    #[test]
    fn security_scope_is_high_even_when_low_path() {
        // security キーワード該当は low_risk_paths 一致よりも優先して high。
        let scope = vec!["docs/security_boundary_notes.md".to_string()];
        let result = assess_risk_tier(&scope, &fda_policy());
        assert_eq!(result.tier, "high");
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("human_required_for")));
    }

    #[test]
    fn empty_scope_is_standard_not_low() {
        let result = assess_risk_tier(&[], &fda_policy());
        assert_eq!(result.tier, "standard");
    }

    #[test]
    fn glob_match_handles_double_star() {
        assert!(glob_match("docs/**", "docs/v1/work_protocol.md"));
        assert!(glob_match(
            "artifacts/runs/**",
            "artifacts/runs/fda-start-1/status.json"
        ));
        assert!(!glob_match("docs/**", "src/lib.rs"));
        assert!(!glob_match("tests/**", "test.rs"));
    }

    #[test]
    fn governance_critical_paths_are_hardcoded() {
        assert!(is_governance_critical_path(".fda/delivery_policy.yaml"));
        assert!(is_governance_critical_path(".fda/gates.yaml"));
        assert!(is_governance_critical_path(
            "scripts/check_review_agent_gate.py"
        ));
        assert!(is_governance_critical_path(
            "scripts/check_architecture_boundaries.py"
        ));
        assert!(is_governance_critical_path(
            "tests/test_review_agent_gate.py"
        ));
        assert!(is_governance_critical_path(".github/workflows/ci.yml"));
        assert!(is_governance_critical_path("src/application/merge.rs"));
        assert!(is_governance_critical_path("src/application/review.rs"));
        assert!(is_governance_critical_path("src/application/risk_tier.rs"));
        assert!(is_governance_critical_path("src/application/policy.rs"));
        assert!(!is_governance_critical_path("docs/v1/work_protocol.md"));
        assert!(!is_governance_critical_path("src/application/gc.rs"));
    }

    fn temp_repo_with_policy(name: &str, policy_body: &str) -> std::path::PathBuf {
        let store = FsArtifactStore;
        let unique = system_unix_seconds();
        let repo = std::env::temp_dir().join(format!("{name}-{unique}"));
        store.create_dir_all(&repo.join(".fda")).unwrap();
        store
            .write_text(&repo.join(".fda").join("delivery_policy.yaml"), policy_body)
            .unwrap();
        repo
    }

    #[test]
    fn hard_guard_denies_forge_relaxation_even_when_policy_marks_fda_low_risk() {
        // upstream 既定のように .fda/** を low_risk_paths に含む policy でも、
        // governance-critical パスの forge_reviewer 緩和はコード側で却下される
        // （YAML では上書き不可）。
        let repo = temp_repo_with_policy(
            "fda-risk-tier-hard-guard",
            "delivery_policy:\n  low_risk_paths:\n    - docs/**\n    - .fda/**\n  human_required_for:\n    - merge_approval\n",
        );
        let changed = vec![".fda/gates.yaml".to_string()];
        let relaxation = proportional_relaxation(&FsArtifactStore, &repo, Some("low"), &changed);
        assert!(!relaxation.forge_reviewer);
        assert!(relaxation.design_qa);
        assert!(relaxation
            .notes
            .iter()
            .any(|note| note.contains("governance-critical") && note.contains("hard guard")));
    }

    #[test]
    fn stored_live_mismatch_denies_relaxation() {
        let repo = temp_repo_with_policy(
            "fda-risk-tier-mismatch",
            "delivery_policy:\n  low_risk_paths:\n    - docs/**\n  human_required_for:\n    - merge_approval\n",
        );
        // 保存 tier は low だが、live の changed_files に範囲外 (src/) が混入。
        let changed = vec!["docs/a.md".to_string(), "src/lib.rs".to_string()];
        let relaxation = proportional_relaxation(&FsArtifactStore, &repo, Some("low"), &changed);
        assert!(!relaxation.forge_reviewer);
        assert!(!relaxation.design_qa);
        assert!(relaxation
            .notes
            .iter()
            .any(|note| note.contains("stored/live tier mismatch")));
    }

    #[test]
    fn relaxation_applies_only_when_stored_and_live_are_both_low() {
        let repo = temp_repo_with_policy(
            "fda-risk-tier-relax-ok",
            "delivery_policy:\n  low_risk_paths:\n    - docs/**\n  human_required_for:\n    - merge_approval\n",
        );
        let changed = vec!["docs/a.md".to_string()];
        // stored 無し → 緩和不適用（fail-closed）。
        let no_stored = proportional_relaxation(&FsArtifactStore, &repo, None, &changed);
        assert!(!no_stored.forge_reviewer && !no_stored.design_qa);
        // stored=low + live=low → 緩和適用、理由に risk_tier=low を含む。
        let relaxed = proportional_relaxation(&FsArtifactStore, &repo, Some("low"), &changed);
        assert!(relaxed.forge_reviewer && relaxed.design_qa);
        assert!(relaxed.reason.as_deref().unwrap().contains("risk_tier=low"));
    }
}
