# Session discovery verification

**Verdict**: FAIL — C1 stale-record diagnostic is missing.
**Profile**: light
**Diff range**: a94b074..907b36f (HEAD 907b36fd646ad8f16845f551ebf9bdcf5b6228b1)
**Round**: 1 — full
**Verifier**: independent sub-agent (author != verifier)

## Sources and scope

Opened `.checks/session-discovery.md`, binding `.design/session-discovery.md`, `PRODUCT.md`, RTK rules, and TLC Implement `references/verify.md`. Source/checklist UI enumeration, Coverage join, Test policy verdicts and fault injection are not required under light; none were substituted with an author report. No Coverage or Test policy section exists in the checklist. Read the full discovery implementation, desktop handlers, polling, smoke, bridge and presentation/rendering code and the named proof bodies. Inspected feature diff file inventory and feature proof additions. Implementation was not changed.

## Checks

| Check | Independently run proof and located evidence | Result |
|---|---|---|
| C1 | `discovery::tests::claude_requires_matching_process_identity` passed. `src-tauri/src/discovery/tests.rs:58-60`: `assert_eq!(report.sessions.len(), 1)`, prefixed ID assertion and missing-process empty assertion prove omission. `src-tauri/src/discovery.rs:206`: `if birth(record.pid) == Some(expected)` has no diagnostic else branch. Independent release report against a scratch registry containing PID 4294967295 returned `0 sessões abertas · descoberta local experimental.` with no stale diagnostic. Test never asserts that diagnostic. | FAIL |
| C2 | `codex_requires_held_lock_and_matching_metadata` passed. `discovery/tests.rs:69`: `assert_eq!(...sessions.len(), 1)` with held lock; :71, :73, :77 `assert!(...sessions.is_empty())` after unlock, for subagent and mismatched ID. `discovery.rs:145` shared lock probe and metadata validation/recheck inspected. | PASS |
| C3 | `identity_and_unknown_state_are_preserved` passed. `discovery/tests.rs:88` asserts 3 sessions; :89-92 asserts common project, `state == "unknown"`, `request.is_none()` and `command.is_none()`; :93-100 asserts 3 distinct IDs. C1's :59 asserts `claude:{A}`; `discovery.rs:88` constructs `{kind}:{id}`. | PASS |
| C4 | `missing_and_bad_sources_are_reported` passed. `discovery/tests.rs:109-112` asserts 3 diagnostics and missing/missing/pending text; :121-123 asserts one surviving session and each malformed-source diagnostic; :125-126 rejects >64 KiB JSON. `held_lock_without_metadata_or_with_unknown_source_is_diagnostic`: :156-157 and :165-166 assert omission plus diagnostic for incompatible/missing metadata. Provider scan branches separately record read errors; traversal skips symlinks and bounds depth/budget. | PASS |
| C5 | `hidden_preferences_survive_restart` and `real_mode_rejects_simulated_responses` passed. `discovery/tests.rs:144-145`: `assert!(sessions[0].hidden)` and `assert!(!sessions[1].hidden)` after fresh `Preferences::load`; :148 `assert!(Preferences::load(path).hidden.is_empty())` after restore. `demo.rs:123-124` loops allow/deny/answer and asserts `data.apply(...).is_err()`. Located real handler `main.rs:243-259` clones preferences, saves before updating authoritative preferences; setup :347-352 reloads Capy config hidden IDs. Native smoke additionally passes `real_responses_rejected`. | PASS |
| C6 | Independent build/typecheck passed. `main.rs:311-315` chooses real except self-test; :353 schedules monitor. `monitor.rs:6` spawns a separate thread; :17-28 rechecks mode, applies current hidden preferences under snapshot lock and replaces shared sessions/integrations; :30 emits existing `demo-updated`; :34 sleeps 5 seconds. `demo_snapshot` exposes same state; bridge subscribes to same event. Independent release `--discover-report` found Claude Code/Prisma and Codex/Capy, both unknown, with 3 provider diagnostics. Native smoke passes frontend readiness of all three windows and `real_discovery_poll_ready`; no UI errors. Runtime exact cadence and agent opening/closing lifecycle remain manual as declared. | PASS within declared proof scope |
| C7 | Independent JS presentation tests, build/typecheck and compiled browser panel preview passed. `presentation.test.mjs:8-12` asserts identity, unknown label, escaping and hide action, and `assert.doesNotMatch(html, /data-action="(?:allow|deny|answer|terminal)"|dangerous/)`; :15-17 asserts unknown -> idle (including alongside done); :20-22 asserts real unavailable/no percentages and demo 38%. Actual `dashboard.ts:14-22` calls the tested row/quota helpers, labels real source and exposes diagnostics. `pet.ts:17` uses tested petState. Independently inspected screenshot + accessibility state at `http://127.0.0.1:4173/panel.html`: explicit demonstration disclosure, demo source controls and simulated quotas; real scenario absent. This browser evidence covers compiled demonstration, not native real rows. Native visual inspection remains manual as declared. | PASS within declared proof scope |

## Finding

1. **C1 diagnostic gap** — `src-tauri/src/discovery.rs:206-212` safely omits stale/reused/dead processes but does not count or disclose them. The checklist expressly requires stale records ignored with a diagnostic. Independent reproduction used a unique ignored scratch directory and a per-process `CLAUDE_CONFIG_DIR` override, leaving agent configuration untouched. It returned ordinary zero-session success text. Add explicit stale/unconfirmed-record diagnostics and assert them for mismatched birth and missing process.

## Gates

- `rtk proxy npm test`: 5 named tests passed, 0 failed.
- `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml`: 13 named tests passed, 0 failed, 0 ignored.
- `rtk npm run build`: typecheck and compiled build passed.
- `rtk npm run verify:native`: 20 named native assertions passed; pet, summary and panel ready; `error: null`, `uiErrors: []`.
- Independent release live report: 2 sessions, Claude Code/Prisma and Codex/Capy, both `unknown`, no request or command; Antigravity pending explicitly.
- Fault injection: not run, light profile.

## Limitations and workspace

Exact runtime polling cadence and external-agent lifecycle transitions, native real-mode visual inspection and physical gestures remain manual. No blanket visual pass is claimed. Native smoke's `real_sessions_unknown` assertion can pass vacuously with no sessions; the separate live report here observed two real sessions. The native smoke script regenerated tracked `.checks/native-smoke.json` (ready-array ordering changed); no implementation edits or commits were made. Diagnostic scratch artifacts are ignored under `/scratch/`. No Capy process remained after verification.
