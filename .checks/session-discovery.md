# Session discovery

Profile: light. Sources: .design/session-discovery.md (binding), PRODUCT.md. One batch, independent verifier required. Base: a94b074.

## Plan

Read local registries in Rust; validate Claude process birth identity with Windows APIs and Codex presence with a nonblocking shared lock probe. Read only bounded metadata, never credentials or conversation bodies. Poll every five seconds off the UI thread. Extend the existing snapshot with source diagnostics. Save hidden IDs in the Capy config directory. Desktop defaults to real; native shell smoke retains isolated demo fixtures.

## Landing

| Decision | Literal shape | Alternative rejected |
|---|---|---|
| Missing activity proof | All discovered sessions use `unknown` | Treat recent file writes or saved idle status as current work |
| Antigravity first stage | Explicit pending diagnostic | Present old presence files as open sessions without proof |
| Source formats | Observed Claude registry and Codex metadata + held lock, experimental | Install hooks or attach/resume external threads before validating |
| Snapshot | Keep current commands/event, `scenario=real`, add `integrations` | Parallel UI state that could disagree with pet |

## Checks

### S1 — Presence and preferences · ~35 KB existing + ~20 KB new; one batch

**C1** Claude records enter only for matching PID/process birth identity; malformed/stale records are ignored with a diagnostic. Proof: `discovery::tests::claude_requires_matching_process_identity`.

**C2** Codex enters only with a held lock and matching metadata; unlocked history and subagents are omitted. Proof: `discovery::tests::codex_requires_held_lock_and_matching_metadata`.

**C3** Discovered sessions use `unknown`, agent-prefixed stable IDs, and retain distinct conversations in the same project. Proof: `discovery::tests::identity_and_unknown_state_are_preserved`.

**C4** Source absence, unreadable/malformed files and pending Antigravity produce explicit diagnostics without failing other providers. Proof: `discovery::tests::missing_and_bad_sources_are_reported`.

**C5** Hidden IDs persist through a fresh tracker and restore, and forged response actions in real mode are refused. Proof: `discovery::tests::hidden_preferences_survive_restart`; `demo::tests::real_mode_rejects_simulated_responses`.

**C6** Desktop starts real; poll updates the shared snapshot every five seconds off the UI thread; demo remains available. Proof: build/typecheck and independent located handler inspection; live diagnostic report `--discover-report` lists currently open local sessions. Native self-test covers frontend readiness and errors. Runtime polling interval/agent lifecycle inspection remains manual.

**C7** Summary/panel disclose the real source, display IDs and unknown status, and offer no simulated terminal or response controls for real rows; real quotas show unavailable. Pet does not render unknown sessions as done. Proof: build/typecheck; independent located rendering inspection and compiled browser preview. Native desktop visual inspection remains manual if unavailable.

## Swept

- Validation/failure: C1/C2/C4; metadata capped at 64 KiB; recursion bounded and symlinks skipped.
- Idempotency/concurrency: poll is serial; real responses refused; agent-prefixed IDs deduplicate; hidden preferences merged under the snapshot lock.
- Authorization: only read source metadata; no credentials, shell frontend permissions or agent configuration edits.
- Data lifecycle: only hidden IDs persist in Capy config; no transcripts copied.
- Dependency failure: sources absent or changed yield diagnostics, never synthetic work.
- State transitions: real/demo switch uses one authoritative snapshot; real unknown maps to idle mascot.
- Observability: local diagnostic report and provider-level diagnostics; native UI errors retain existing reporting.

## Handoff

One batch under 150k tokens; no builder delegation. Verifier receives full base..HEAD and this checklist.
