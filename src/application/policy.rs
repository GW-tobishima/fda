//! F1 判断の立法化（delegation contract）。
//!
//! `fda policy propose` は全 run の human_decision_packet + decision_receipts を走査し、
//! (decision type × summary の正規化署名 × answer) でクラスタして、min-occurrences 以上の
//! 同型判断を「委任契約の候補」として `policy_proposal.{json,md}` に**提案するだけ**である。
//! **`.fda` へは絶対に書かない**。契約の制定は人間が `.fda/delegation_contract.yaml` を
//! 編集・追記する行為のみで、AI は提案までしか行わない（自己制定・自動適用の禁止）。
//!
//! 併せて delegation contract の読取・照合ヘルパ（`fda decide --by-contract` /
//! `fda status` の適用可ヒント）を提供する。適用は明示指定時だけ、かつ fail-closed。

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

use crate::application::decisions::{
    read_decision_receipts, read_json_value, recorded_decision_receipts_from_packet, value_string,
    value_string_array,
};
use crate::application::ports::ArtifactStore;
use crate::cli::args::PolicyConfig;
use crate::infra::clock::system_unix_seconds;
use crate::infra::fs_store::{list_dir_names, FsArtifactStore};
use crate::support::date::is_valid_ymd;
use crate::support::paths::{display_path, resolve_path};

const POLICY_PROPOSAL_SCHEMA_VERSION: &str = "fda.policy_proposal.v1";
/// 正規化署名で先頭から見る最大文字数（記号・数字を除去後）。素朴実装で十分。
const SIGNATURE_PREFIX_CHARS: usize = 40;
/// 1 候補に載せる代表 summary（keyword 候補）の最大件数。
const MAX_KEYWORDS_PER_CANDIDATE: usize = 3;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PolicyCandidate {
    pub(crate) proposed_rule_id: String,
    pub(crate) decision_type: String,
    pub(crate) answer: String,
    pub(crate) occurrences: usize,
    pub(crate) match_summary_keywords: Vec<String>,
    pub(crate) enacted_from: Vec<String>,
    pub(crate) summary_signature: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct PolicyProposalResult {
    pub(crate) schema_version: &'static str,
    pub(crate) verdict: &'static str,
    pub(crate) artifacts_root: String,
    pub(crate) proposal_path: String,
    pub(crate) proposal_markdown_path: String,
    pub(crate) scanned_runs: usize,
    pub(crate) candidate_count: usize,
    pub(crate) min_occurrences: u64,
    pub(crate) candidates: Vec<PolicyCandidate>,
    pub(crate) next_actions: Vec<String>,
}

/// クラスタ集計用の内部アキュムレータ（IO を分離した純データ）。
struct ClusterAcc {
    decision_type: String,
    answer: String,
    signature: String,
    occurrences: usize,
    summaries: Vec<String>,
    enacted_from: Vec<String>,
}

pub(crate) fn policy_propose(config: &PolicyConfig) -> Result<PolicyProposalResult, String> {
    let store = FsArtifactStore;
    policy_propose_with(config, &store, system_unix_seconds())
}

fn policy_propose_with(
    config: &PolicyConfig,
    store: &impl ArtifactStore,
    now_unix: u64,
) -> Result<PolicyProposalResult, String> {
    if config.min_occurrences == 0 {
        return Err("--min-occurrences must be >= 1".to_string());
    }
    let repo_root = store.canonicalize(&config.repo_root).map_err(|e| {
        format!(
            "failed to resolve repo root {}: {e}",
            config.repo_root.display()
        )
    })?;
    let artifacts_root = resolve_path(&repo_root, &config.artifacts_root);
    let artifacts_root_display = display_path(&repo_root, &artifacts_root);

    let run_names = if store.exists(&artifacts_root) {
        list_dir_names(&artifacts_root)?
    } else {
        Vec::new()
    };

    let mut clusters: BTreeMap<(String, String, String), ClusterAcc> = BTreeMap::new();
    let mut scanned_runs = 0usize;
    for name in &run_names {
        // `_policy` / `_gc` などの内部ディレクトリは走査対象外。
        if name.starts_with('_') {
            continue;
        }
        scanned_runs += 1;
        let run_dir = artifacts_root.join(name);
        accumulate_run(store, name, &run_dir, &mut clusters)?;
    }

    let mut candidates = Vec::new();
    for acc in clusters.values() {
        if acc.occurrences < config.min_occurrences as usize {
            continue;
        }
        candidates.push(acc);
    }
    // 出現回数の多い順、同数は署名順で安定化。
    candidates.sort_by(|a, b| {
        b.occurrences
            .cmp(&a.occurrences)
            .then_with(|| a.signature.cmp(&b.signature))
    });
    let candidates: Vec<PolicyCandidate> = candidates
        .into_iter()
        .enumerate()
        .map(|(index, acc)| PolicyCandidate {
            proposed_rule_id: format!("DC-PROPOSED-{:03}", index + 1),
            decision_type: acc.decision_type.clone(),
            answer: acc.answer.clone(),
            occurrences: acc.occurrences,
            match_summary_keywords: acc
                .summaries
                .iter()
                .take(MAX_KEYWORDS_PER_CANDIDATE)
                .cloned()
                .collect(),
            enacted_from: acc.enacted_from.clone(),
            summary_signature: acc.signature.clone(),
        })
        .collect();

    let out_dir = resolve_path(&repo_root, &config.out);
    store.create_dir_all(&out_dir).map_err(|e| {
        format!(
            "failed to create policy proposal dir {}: {e}",
            out_dir.display()
        )
    })?;
    let proposal_path = out_dir.join("policy_proposal.json");
    store.write_json(
        &proposal_path,
        &proposal_value(
            now_unix,
            &artifacts_root_display,
            config.min_occurrences,
            scanned_runs,
            &candidates,
        ),
    )?;
    let proposal_markdown_path = out_dir.join("policy_proposal.md");
    store.write_text(
        &proposal_markdown_path,
        &proposal_markdown(
            now_unix,
            &artifacts_root_display,
            config.min_occurrences,
            scanned_runs,
            &candidates,
        ),
    )?;

    let candidate_count = candidates.len();
    Ok(PolicyProposalResult {
        schema_version: "fda.policy_proposal_result.v0",
        verdict: "pass",
        artifacts_root: artifacts_root_display,
        proposal_path: display_path(&repo_root, &proposal_path),
        proposal_markdown_path: display_path(&repo_root, &proposal_markdown_path),
        scanned_runs,
        candidate_count,
        min_occurrences: config.min_occurrences,
        next_actions: proposal_next_actions(candidate_count),
        candidates,
    })
}

fn accumulate_run(
    store: &impl ArtifactStore,
    run: &str,
    run_dir: &Path,
    clusters: &mut BTreeMap<(String, String, String), ClusterAcc>,
) -> Result<(), String> {
    let packet_path = run_dir.join("human_decision_packet.json");
    if !store.exists(&packet_path) {
        return Ok(());
    }
    let packet = read_json_value(store, &packet_path)?;

    // decision_id -> (type, summary)。type / summary が揃う nested decision のみ対象。
    let mut decisions: BTreeMap<String, (String, String)> = BTreeMap::new();
    for decision in packet
        .get("decisions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(decision_id) = value_string(decision, "decision_id") else {
            continue;
        };
        let Some(decision_type) = value_string(decision, "type") else {
            continue;
        };
        let Some(summary) = value_string(decision, "summary") else {
            continue;
        };
        decisions.insert(decision_id, (decision_type, summary));
    }
    if decisions.is_empty() {
        return Ok(());
    }

    // answers: decision_receipts.json を正、無ければ packet の recorded_decision を補完。
    let mut receipts = read_decision_receipts(store, &run_dir.join("decision_receipts.json"))?;
    for (decision_id, receipt) in recorded_decision_receipts_from_packet(&packet) {
        receipts.entry(decision_id).or_insert(receipt);
    }

    for (decision_id, (decision_type, summary)) in decisions {
        let Some(answer) = receipts
            .get(&decision_id)
            .and_then(|receipt| value_string(receipt, "answer"))
        else {
            continue;
        };
        let signature = normalize_summary_signature(&summary);
        let key = (decision_type.clone(), answer.clone(), signature.clone());
        let acc = clusters.entry(key).or_insert_with(|| ClusterAcc {
            decision_type: decision_type.clone(),
            answer: answer.clone(),
            signature: signature.clone(),
            occurrences: 0,
            summaries: Vec::new(),
            enacted_from: Vec::new(),
        });
        acc.occurrences += 1;
        let trimmed = summary.trim().to_string();
        if !trimmed.is_empty() && !acc.summaries.contains(&trimmed) {
            acc.summaries.push(trimmed);
        }
        acc.enacted_from.push(format!("{run} {decision_id}"));
    }
    Ok(())
}

/// 記号・数字・空白を除去し、残った文字（英字・CJK 等）の先頭 N 文字を署名にする素朴実装。
/// `fda ui` の precedent 照合（同 type 判断の正規化署名類似）でも再利用する。
pub(crate) fn normalize_summary_signature(summary: &str) -> String {
    summary
        .chars()
        .filter(|ch| ch.is_alphabetic())
        .map(|ch| ch.to_ascii_lowercase())
        .take(SIGNATURE_PREFIX_CHARS)
        .collect()
}

fn proposal_value(
    now_unix: u64,
    artifacts_root_display: &str,
    min_occurrences: u64,
    scanned_runs: usize,
    candidates: &[PolicyCandidate],
) -> Value {
    json!({
        "schema_version": POLICY_PROPOSAL_SCHEMA_VERSION,
        "generated_at_unix": now_unix,
        "artifacts_root": artifacts_root_display,
        "min_occurrences": min_occurrences,
        "scanned_runs": scanned_runs,
        "candidate_count": candidates.len(),
        "candidates": candidates
            .iter()
            .map(|candidate| json!({
                "proposed_rule_id": candidate.proposed_rule_id,
                "decision_type": candidate.decision_type,
                "answer": candidate.answer,
                "occurrences": candidate.occurrences,
                "match_summary_keywords": candidate.match_summary_keywords,
                "enacted_from": candidate.enacted_from,
                "summary_signature": candidate.summary_signature,
            }))
            .collect::<Vec<_>>(),
    })
}

fn proposal_markdown(
    now_unix: u64,
    artifacts_root_display: &str,
    min_occurrences: u64,
    scanned_runs: usize,
    candidates: &[PolicyCandidate],
) -> String {
    let mut body = String::new();
    body.push_str("# Policy Proposal（委任契約の逆提案）\n\n");
    body.push_str(&format!("- artifacts-root: `{artifacts_root_display}`\n"));
    body.push_str(&format!("- generated_at_unix: {now_unix}\n"));
    body.push_str(&format!("- min-occurrences: {min_occurrences}\n"));
    body.push_str(&format!("- スキャンした run 数: {scanned_runs}\n"));
    body.push_str(&format!("- 契約候補: {} 件\n\n", candidates.len()));
    body.push_str("**これは提案です。`fda policy propose` は `.fda` へ一切書き込みません。**\n");
    body.push_str(
        "**制定は人間が下記 YAML スニペットを `.fda/delegation_contract.yaml` へ編集・追記する行為のみです**\n",
    );
    body.push_str(
        "（`expires` 必須・`authority` は承認権限を持つ人間を記入。AI は制定・自動適用をしません）。\n\n",
    );

    if candidates.is_empty() {
        body.push_str(&format!(
            "min-occurrences {min_occurrences} 以上の同型判断は見つかりませんでした。\n"
        ));
        return body;
    }

    for candidate in candidates {
        body.push_str(&format!(
            "## {}（{} 回・type: {} / answer: {}）\n\n",
            candidate.proposed_rule_id,
            candidate.occurrences,
            candidate.decision_type,
            candidate.answer
        ));
        body.push_str("由来（enacted_from 候補）:\n\n");
        for source in &candidate.enacted_from {
            body.push_str(&format!("- `{source}`\n"));
        }
        body.push_str("\n`.fda/delegation_contract.yaml` へ追記する YAML スニペット:\n\n");
        body.push_str("```yaml\n");
        body.push_str(&candidate_yaml_snippet(candidate));
        body.push_str("```\n\n");
    }
    body
}

fn candidate_yaml_snippet(candidate: &PolicyCandidate) -> String {
    let mut snippet = String::new();
    snippet.push_str(&format!("  - rule_id: {}\n", candidate.proposed_rule_id));
    snippet.push_str(&format!(
        "    decision_type: {}\n",
        yaml_double_quote(&candidate.decision_type)
    ));
    snippet.push_str("    match_summary_keywords:\n");
    for keyword in &candidate.match_summary_keywords {
        snippet.push_str(&format!("      - {}\n", yaml_double_quote(keyword)));
    }
    snippet.push_str(&format!(
        "    answer: {}\n",
        yaml_double_quote(&candidate.answer)
    ));
    snippet.push_str("    authority: \"<承認権限を持つ人間>\"\n");
    snippet.push_str("    enacted_from:\n");
    for source in &candidate.enacted_from {
        snippet.push_str(&format!("      - {}\n", yaml_double_quote(source)));
    }
    snippet.push_str("    expires: \"<YYYY-MM-DD 必須>\"\n");
    snippet.push_str(&format!(
        "    note: {}\n",
        yaml_double_quote(&format!(
            "同型判断が {} 回承認された履歴からの逆提案",
            candidate.occurrences
        ))
    ));
    snippet
}

fn yaml_double_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn proposal_next_actions(candidate_count: usize) -> Vec<String> {
    if candidate_count == 0 {
        return vec!["委任契約の候補はありません（min-occurrences 未満）。".to_string()];
    }
    vec![
        format!("policy_proposal.md を確認する（候補 {candidate_count} 件）"),
        ".fda/delegation_contract.yaml へ人間が編集・追記して契約を制定する（AI は制定しない）"
            .to_string(),
    ]
}

// --- delegation contract の読取・照合（decide --by-contract / status hint 用） ---

/// 適用可能と判定された契約の実体（decide --by-contract の回答記録に使う）。
#[derive(Debug)]
pub(crate) struct ContractApplication {
    pub(crate) rule_id: String,
    pub(crate) answer: String,
    pub(crate) authority: String,
    pub(crate) expires: String,
}

/// `.fda/delegation_contract.yaml` を読み、`delegation_contract` 配列（ルール群）を返す。
/// ファイルが無ければ空配列。**YAML 自体が壊れている / root 構造が不正なら fail-closed で Err**。
pub(crate) fn read_delegation_contract_rules(
    store: &impl ArtifactStore,
    yaml: &impl crate::application::ports::YamlValidator,
    repo_root: &Path,
) -> Result<Vec<Value>, String> {
    let path = repo_root.join(".fda").join("delegation_contract.yaml");
    if !store.exists(&path) {
        return Ok(Vec::new());
    }
    let body = store.read_text(&path)?;
    let value = yaml.parse_yaml_value(&path, &body)?;
    let rules = value
        .get("delegation_contract")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "{} の root は delegation_contract 配列である必要があります",
                display_path(repo_root, &path)
            )
        })?;
    Ok(rules.clone())
}

/// `fda decide --by-contract <rule_id>` の評価。全条件を満たす場合のみ Ok。
/// 1 つでも満たさなければ、人間判断へ戻す明示メッセージ付きの Err を返す（fail-closed）。
/// 他ルールが不正でも、対象 rule のみを見るため巻き込みはしない。
pub(crate) fn evaluate_contract_for_decision(
    rules: &[Value],
    rule_id: &str,
    decision_id: &str,
    decision_type: &str,
    decision_summary: &str,
    today_ymd: &str,
) -> Result<ContractApplication, String> {
    let Some(rule) = rules
        .iter()
        .find(|rule| value_string(rule, "rule_id").as_deref() == Some(rule_id))
    else {
        return Err(format!(
            "{rule_id} は .fda/delegation_contract.yaml に存在しません。fda decide {decision_id} --answer <答え> で人間が回答してください。"
        ));
    };

    let contract_type = value_string(rule, "decision_type")
        .ok_or_else(|| reject(rule_id, decision_id, "decision_type が未設定です"))?;
    let answer = value_string(rule, "answer")
        .ok_or_else(|| reject(rule_id, decision_id, "answer が未設定です"))?;
    let authority = value_string(rule, "authority")
        .ok_or_else(|| reject(rule_id, decision_id, "authority が未設定です"))?;
    let expires = value_string(rule, "expires")
        .ok_or_else(|| reject(rule_id, decision_id, "expires が未設定です"))?;
    let keywords = value_string_array(rule, "match_summary_keywords");
    if keywords.is_empty() {
        return Err(reject(
            rule_id,
            decision_id,
            "match_summary_keywords が空です",
        ));
    }
    if !is_valid_ymd(&expires) {
        return Err(reject(
            rule_id,
            decision_id,
            &format!("expires `{expires}` が YYYY-MM-DD 形式ではありません"),
        ));
    }
    if contract_type != decision_type {
        return Err(reject(
            rule_id,
            decision_id,
            &format!(
                "decision_type `{contract_type}` が対象判断の type `{decision_type}` と一致しません"
            ),
        ));
    }
    if !keywords
        .iter()
        .any(|keyword| !keyword.is_empty() && decision_summary.contains(keyword.as_str()))
    {
        return Err(reject(
            rule_id,
            decision_id,
            "match_summary_keywords のいずれも対象判断の summary に含まれません",
        ));
    }
    if expires.as_str() < today_ymd {
        return Err(format!(
            "{rule_id} は expires ({expires}) 切れのため適用できません。fda decide {decision_id} --answer <答え> で人間が回答してください。"
        ));
    }
    Ok(ContractApplication {
        rule_id: rule_id.to_string(),
        answer,
        authority,
        expires,
    })
}

fn reject(rule_id: &str, decision_id: &str, reason: &str) -> String {
    format!(
        "{rule_id} は適用できません（{reason}）。fda decide {decision_id} --answer <答え> で人間が回答してください。"
    )
}

/// `fda status` のヒント用。対象判断に適用可能な rule_id を返す（無効ルールは黙ってスキップ）。
pub(crate) fn applicable_rule_ids(
    rules: &[Value],
    decision_type: &str,
    decision_summary: &str,
    today_ymd: &str,
) -> Vec<String> {
    rules
        .iter()
        .filter_map(|rule| {
            let rule_id = value_string(rule, "rule_id")?;
            if rule_applies(rule, decision_type, decision_summary, today_ymd) {
                Some(rule_id)
            } else {
                None
            }
        })
        .collect()
}

fn rule_applies(
    rule: &Value,
    decision_type: &str,
    decision_summary: &str,
    today_ymd: &str,
) -> bool {
    let Some(contract_type) = value_string(rule, "decision_type") else {
        return false;
    };
    if contract_type != decision_type {
        return false;
    }
    if value_string(rule, "answer").is_none() || value_string(rule, "authority").is_none() {
        return false;
    }
    let Some(expires) = value_string(rule, "expires") else {
        return false;
    };
    if !is_valid_ymd(&expires) || expires.as_str() < today_ymd {
        return false;
    }
    let keywords = value_string_array(rule, "match_summary_keywords");
    keywords
        .iter()
        .any(|keyword| !keyword.is_empty() && decision_summary.contains(keyword.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = system_unix_seconds();
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        // scan 汚染を避けるため、run を書き込む repo dir は毎回一意にする。
        let dir = std::env::temp_dir().join(format!("{name}-{unique}-{seq}"));
        FsArtifactStore.create_dir_all(&dir).unwrap();
        dir
    }

    fn write_same_type_decision(runs_root: &Path, run: &str, answer: &str) {
        let store = FsArtifactStore;
        let run_dir = runs_root.join(run);
        store.create_dir_all(&run_dir).unwrap();
        store
            .write_json(
                &run_dir.join("human_decision_packet.json"),
                &json!({
                    "decisions": [{
                        "decision_id": "HD-FDA-001",
                        "type": "spec_decision",
                        "summary": "入力から抽出した Scope In / Scope Out を Intake 正本として固定してよいか",
                        "options": [{"id": "yes"}, {"id": "no"}],
                        "recommended_option_id": "yes",
                        "required_before": "Design Gate"
                    }]
                }),
            )
            .unwrap();
        store
            .write_json(
                &run_dir.join("decision_receipts.json"),
                &json!({
                    "schema_version": "fda.decision_receipts.v0",
                    "receipts": [{"decision_id": "HD-FDA-001", "answer": answer}]
                }),
            )
            .unwrap();
    }

    fn propose(repo: &Path, min: u64) -> PolicyProposalResult {
        let config = PolicyConfig {
            repo_root: repo.to_path_buf(),
            artifacts_root: PathBuf::from("artifacts/runs"),
            out: PathBuf::from("artifacts/runs/_policy"),
            min_occurrences: min,
            print_json: false,
        };
        policy_propose_with(&config, &FsArtifactStore, 1_783_555_200).unwrap()
    }

    #[test]
    fn three_same_type_decisions_become_a_candidate() {
        let repo = temp_dir("fda-policy-three");
        let runs = repo.join("artifacts").join("runs");
        for run in ["run-a", "run-b", "run-c"] {
            write_same_type_decision(&runs, run, "approve_scope");
        }
        let result = propose(&repo, 3);
        assert_eq!(result.candidate_count, 1);
        let candidate = &result.candidates[0];
        assert_eq!(candidate.decision_type, "spec_decision");
        assert_eq!(candidate.answer, "approve_scope");
        assert_eq!(candidate.occurrences, 3);
        assert_eq!(candidate.enacted_from.len(), 3);
        assert!(!candidate.match_summary_keywords.is_empty());
    }

    #[test]
    fn two_occurrences_do_not_reach_min_and_are_not_candidates() {
        let repo = temp_dir("fda-policy-two");
        let runs = repo.join("artifacts").join("runs");
        for run in ["run-a", "run-b"] {
            write_same_type_decision(&runs, run, "approve_scope");
        }
        let result = propose(&repo, 3);
        assert_eq!(result.candidate_count, 0);
        assert!(result.candidates.is_empty());
    }

    #[test]
    fn propose_never_writes_into_dot_fda() {
        let repo = temp_dir("fda-policy-nowrite");
        let runs = repo.join("artifacts").join("runs");
        for run in ["run-a", "run-b", "run-c"] {
            write_same_type_decision(&runs, run, "approve_scope");
        }
        let result = propose(&repo, 3);
        assert_eq!(result.candidate_count, 1);
        // 提案は out ディレクトリにだけ出力され、.fda は触れられない。
        assert!(!repo.join(".fda").exists());
        assert!(repo
            .join("artifacts/runs/_policy/policy_proposal.json")
            .exists());
        assert!(repo
            .join("artifacts/runs/_policy/policy_proposal.md")
            .exists());
    }

    #[test]
    fn differing_answers_do_not_cluster_together() {
        let repo = temp_dir("fda-policy-diff-answer");
        let runs = repo.join("artifacts").join("runs");
        write_same_type_decision(&runs, "run-a", "approve_scope");
        write_same_type_decision(&runs, "run-b", "approve_scope");
        write_same_type_decision(&runs, "run-c", "reject_scope");
        let result = propose(&repo, 3);
        // 3 件あるが answer が割れているので min=3 のクラスタは無い。
        assert_eq!(result.candidate_count, 0);
    }

    fn contract_rules() -> Vec<Value> {
        vec![json!({
            "rule_id": "DC-001",
            "decision_type": "spec_decision",
            "match_summary_keywords": ["Scope In / Scope Out", "Intake 正本"],
            "answer": "approve_scope",
            "authority": "k_tobishima",
            "enacted_from": ["run HD-FDA-001"],
            "expires": "2026-10-01"
        })]
    }

    const SUMMARY: &str =
        "入力から抽出した Scope In / Scope Out を Intake 正本として固定してよいか";

    #[test]
    fn contract_applies_when_all_conditions_met() {
        let app = evaluate_contract_for_decision(
            &contract_rules(),
            "DC-001",
            "HD-FDA-001",
            "spec_decision",
            SUMMARY,
            "2026-07-09",
        )
        .unwrap();
        assert_eq!(app.answer, "approve_scope");
        assert_eq!(app.authority, "k_tobishima");
        assert_eq!(app.expires, "2026-10-01");
    }

    #[test]
    fn contract_rejected_when_expired() {
        let err = evaluate_contract_for_decision(
            &contract_rules(),
            "DC-001",
            "HD-FDA-001",
            "spec_decision",
            SUMMARY,
            "2026-10-02",
        )
        .unwrap_err();
        assert!(err.contains("expires"));
        assert!(err.contains("fda decide HD-FDA-001 --answer"));
    }

    #[test]
    fn contract_rejected_on_type_mismatch() {
        let err = evaluate_contract_for_decision(
            &contract_rules(),
            "DC-001",
            "HD-FDA-001",
            "risk_decision",
            SUMMARY,
            "2026-07-09",
        )
        .unwrap_err();
        assert!(err.contains("decision_type"));
    }

    #[test]
    fn contract_rejected_on_keyword_mismatch() {
        let err = evaluate_contract_for_decision(
            &contract_rules(),
            "DC-001",
            "HD-FDA-001",
            "spec_decision",
            "まったく無関係な判断内容",
            "2026-07-09",
        )
        .unwrap_err();
        assert!(err.contains("match_summary_keywords"));
    }

    #[test]
    fn contract_rejected_when_rule_missing() {
        let err = evaluate_contract_for_decision(
            &contract_rules(),
            "DC-999",
            "HD-FDA-001",
            "spec_decision",
            SUMMARY,
            "2026-07-09",
        )
        .unwrap_err();
        assert!(err.contains("存在しません"));
    }

    #[test]
    fn applicable_rule_ids_lists_valid_rule_and_skips_expired() {
        let ids = applicable_rule_ids(&contract_rules(), "spec_decision", SUMMARY, "2026-07-09");
        assert_eq!(ids, vec!["DC-001".to_string()]);
        let none = applicable_rule_ids(&contract_rules(), "spec_decision", SUMMARY, "2026-10-02");
        assert!(none.is_empty());
    }

    #[test]
    fn invalid_rule_is_skipped_without_blocking_others() {
        let mut rules = contract_rules();
        // expires 無しの壊れたルールを混ぜても、有効な DC-001 は生き残る。
        rules.push(json!({
            "rule_id": "DC-BROKEN",
            "decision_type": "spec_decision",
            "match_summary_keywords": ["Scope In / Scope Out"],
            "answer": "approve_scope",
            "authority": "k_tobishima",
            "enacted_from": ["run X"]
        }));
        let ids = applicable_rule_ids(&rules, "spec_decision", SUMMARY, "2026-07-09");
        assert_eq!(ids, vec!["DC-001".to_string()]);
    }
}
