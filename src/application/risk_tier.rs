//! F4 比例ゲート: 変更の risk tier を判定する純ロジックと成果物出力。
//!
//! `assess_risk_tier` は scope パス群と delivery_policy（parsed）だけを見る純関数で、
//! low / standard / high を返す。merge gate はこの tier を読み、low の run では
//! forge_reviewer / design_qa の conditional 要求を「auto not_applicable（理由記録）」に
//! 軽量化する（ゲートの種類は減らさない = fail-closed 維持）。

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
}
