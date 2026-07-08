# Implementation Handoff

artifact: implementation_handoff
schema_version: fda.implementation_handoff.v0
status: complete

## 1. Purpose

PR-V1-018 は current Codex CLI primary を実装者として使い、PR #87 と actual PR の対応、test evidence、implementation receipt、external PR receipt を残す。

## 2. Context

- Program: FDA-V1
- Epic: EPIC-FDA-V1-OPERATIONAL
- Source artifact dir: docs/standards/delivery-artifacts-v0/examples/fda_v1_operational_e2e
- Target repo cwd: /root/code/forge-delivery-agent
- Implementation actor: current Codex CLI session
- Actual PR: https://github.com/msamunetogetoge/forge-delivery-agent/pull/87

## 3. Scope In

- HDP-001からHDP-008までの人間判断Aを前提に、Codex CLI primary rebaselineを実装する。
- 必須 `.fda` profile gate と Review Agent Gate packet reflection gateを実装する。
- target repo の実装、test、PR作成を current Codex CLI session で行う。
- 実装結果を `implementation_receipt.json` と `external_pr_receipt.json` に正規化する。

## 4. Scope Out

- Functional QA / Security QA は PR-V1-007 に分離する。
- merge / release / human-only approval は行わない。
- Codex MCP live implementerをV1主経路へ戻さない。
- Review Agent Gate、CI、Codex review、Human merge approvalを自己承認しない。

## 5. Expected Evidence

- `live_execution_evidence.json`
- `implementation_receipt.json`
- `external_pr_receipt.json`
- `coding_agent_thread_state.json`
- `artifacts/runs/v1-pivot-011-operational-v1-codex-cli-primary-finalize-20260629/validation_report.json`
