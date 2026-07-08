# ATO HQ UIUX Knowledge Pack

This directory stores historical Knowledge entries captured during the ATO-HQ / runner / Codex CLI overlap review.

These entries are not task logs. They are reusable product and UIUX constraints for future AI delivery surfaces.

## Entries

| ID | Type | Summary | Primary implication |
|---|---|---|---|
| `kn_01KVKG7FG9XVA3PEF7GYTNDXPN` | pattern | 通常の人間向け UI で ATO の Task/Run/Agent 粒度を主表示にしない。 | Human-facing surfaces should default to artifacts, decisions, risks, outcomes, and evidence; Task/Run/Agent details are drill-down only. |
| `kn_01KVKG7FH49649SBZEBX9CB4SR` | pattern | 人間に AI 実行用パケットを作らせる UIUX は避ける。 | Humans provide intent, constraints, success criteria, and things to avoid; the system generates Job Packet / Work Contract / Trace Keys. |
| `kn_01KVKG7FJ0GHA1E6HT4EW86K0S` | insight | Codex CLI が自然な入口なら、HQ 型 UI は CLI にない価値だけを担う。 | Codex CLI can remain the primary hands-on entry point; HQ/Mission Control should focus on backlog, multi-work oversight, review, submission, audit, and Human Turn aggregation. |
| `kn_01KVKG7FJGQB6XAB7Y1825KJ7F` | failure | HQ から runner/Codex を再起動しても、何が起きているか直感できなければ信頼されない。 | Retry/resume UI must explain phase, diff from previous run, actor, input, stop condition, next action, and evidence. |
| `kn_01KVKGFHGCEVG0MS3VJ6C02NNX` | insight | CLI 主入口でも、生成 artifact の即時閲覧導線は UI が持つ明確な価値になる。 | Provide Output Hub / Artifact Inbox for generated docs, reports, previews, editor/browser open links, diffs, and evidence. |

## Product constraints for forge-delivery-agent

1. **Do not make internal execution the default human view.**
   - Program/Epic/Case/Artifact/Decision are default units.
   - Task/Run/Agent are audit and debug drill-down units.

2. **Do not ask humans to author execution packets.**
   - The runtime generates Job Packet / Work Contract / Trace Keys from natural-language intent.
   - Humans review missing facts, risk acceptance, scope changes, and final submission readiness.

3. **Respect Codex CLI as the primary hands-on entry point.**
   - UI should not compete with CLI for simple implementation intake.
   - UI should focus on things CLI does poorly: backlog, multi-work oversight, artifact browsing, asynchronous review, customer/company-facing accountability, audit, and Human Turn aggregation.

4. **Provide artifact-first value.**
   - The human-facing surface should expose generated artifacts immediately.
   - At minimum: latest artifact list, grouping by type, preview, open-in-editor/browser link, diff link, and evidence link.

5. **Make runner/retry/resume explainable.**
   - No opaque "run again" button.
   - Every retry/resume action must show why it is needed, what input changed, who/what will run, where it will stop, what happens next, and which evidence will prove completion or failure.

## Affected surfaces

- `docs/runtime/ai_delivery_runtime.md`
- `agents/human_liaison.md`
- `adapters/README.md`
- `evals/README.md`
- Future ATO/Forge Mission Control and Output Hub designs

## Review policy

Knowledge entries are historical artifacts. Do not rewrite the entry contents in-place. Add new Knowledge entries for corrections or changed understanding.
