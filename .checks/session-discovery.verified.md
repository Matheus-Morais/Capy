# Session discovery verification

**Verdict**: PASS — 7/7 checks proven within their declared proof scope.
**Profile**: light
**Diff range**: a94b074..b7860f1 (HEAD b7860f1b767b3735c9bdc477921aadce3243cf38)
**Fix range**: 907b36f..b7860f1
**Round**: 2 — scoped
**Verifier**: independent sub-agent (author != verifier)

## Sources and scope

Binding `.design/session-discovery.md`, `PRODUCT.md`, checklist `.checks/session-discovery.md`, RTK rules and TLC Implement `references/verify.md` were opened in round 1 at 907b36f; source and scope inspection carried from that commit. No binding source or checklist changed in the fix. The fix modifies discarded-record diagnostic text and the Claude unmatched-process branch plus related assertions. C1 and affected C4 have been reverified; unaffected checks carry from 907b36f with refreshed citations in the touched test file. All proof gates were independently rerun at b7860f1. Source/checklist UI enumeration, Coverage joins, Test policy verdicts and fault injection are not required under light. No Coverage or Test policy section exists in the checklist. No implementation edits or commits made by verifier.

## Checks

| Check | Evidence | Result |
|---|---|---|
| C1 | **Verified at b7860f1.** `discovery::tests::claude_requires_matching_process_identity` passed. `src-tauri/src/discovery/tests.rs:58-59` asserts one session and `claude:{A}`; :60-62 asserts `.contains("1 registro descartado")` for mismatched process birth; :64-67 asserts no sessions and `.contains("2 registros descartados")` for missing processes. `src-tauri/src/discovery.rs:217` increments problems when birth does not match. Independent rebuilt-release report with nonexistent PID 4294967295 returned `0 sessões abertas · 1 registro descartado (presença não confirmada ou leitura incompatível). Fonte experimental.` | PASS |
| C2 | **Carried from 907b36f; citations refreshed at b7860f1.** `codex_requires_held_lock_and_matching_metadata` reran and passed. `discovery/tests.rs:76` asserts 1 with held lock; :78, :80, :84 assert empty after unlock, for subagent and mismatched ID. `discovery.rs:142-148` nonblocking shared probe unchanged. | PASS |
| C3 | **Carried from 907b36f; citations refreshed at b7860f1.** `identity_and_unknown_state_are_preserved` reran and passed. `discovery/tests.rs:95` asserts 3 sessions; :96-99 asserts common project, `state == "unknown"`, `request.is_none()` and `command.is_none()`; :100-107 asserts 3 distinct IDs. C1 :59 asserts `claude:{A}`; `discovery.rs:88` constructs `{kind}:{id}` unchanged. | PASS |
| C4 | **Verified at b7860f1.** `missing_and_bad_sources_are_reported` reran and passed. `discovery/tests.rs:116-119` asserts 3 diagnostics and missing/missing/pending text; :128-134 asserts one surviving session and `.contains("1 registro descartado")` for each malformed provider; :136-137 rejects >64 KiB JSON. `held_lock_without_metadata_or_with_unknown_source_is_diagnostic` reran and passed; :167-170 and :178-181 assert empty plus one discarded-record diagnostic for incompatible/missing metadata. Shared message now explicitly includes unconfirmed presence or incompatible read (`discovery.rs:166-172`). Unchanged read-error, independent-provider and bounded traversal branches carry from 907b36f. | PASS |
| C5 | **Carried from 907b36f; citations refreshed at b7860f1.** `hidden_preferences_survive_restart` and `real_mode_rejects_simulated_responses` reran and passed. `discovery/tests.rs:155-156` asserts first hidden and second visible after fresh load; :159 `assert!(Preferences::load(path).hidden.is_empty())` after restore. `demo.rs:123-124` loops allow/deny/answer and asserts `data.apply(...).is_err()`. `main.rs:243-259` saves cloned preferences before replacing shared preferences; :347-352 loads Capy config hidden IDs. Native smoke again passes real response rejection. | PASS |
| C6 | **Handler/lifecycle inspection carried from 907b36f; build/native gates verified at b7860f1.** `main.rs:311-315` starts real except self-test; :353 schedules monitor. `monitor.rs:6` spawns off UI thread; :17-28 rechecks mode, applies current hidden preferences under shared snapshot lock and replaces sessions/integrations; :30 emits `demo-updated`; :34 sleeps 5 seconds. Native smoke again passes all frontend readiness and `real_discovery_poll_ready`, no UI errors. Round-1 independent local report observed Claude Code/Prisma and Codex/Capy unknown; round-2 controlled release report still found real Codex/Capy while isolating Claude stale source. Exact cadence and external-agent opening/closing remain manual as declared. | PASS within declared proof scope |
| C7 | **Rendering inspection and compiled demo preview carried from 907b36f; JS/build/native gates verified at b7860f1.** `presentation.test.mjs:8-12` asserts identity, unknown, escaped project, hide action and `assert.doesNotMatch(html, /data-action="(?:allow|deny|answer|terminal)"|dangerous/)`; :15-17 asserts unknown -> idle including beside done; :20-22 asserts unavailable real quotas/no percentages and demo 38%. Actual `dashboard.ts:14-22` calls tested helpers, discloses source and provider diagnostics; `pet.ts:17` uses petState. Round-1 independent screenshot/accessibility at `http://127.0.0.1:4173/panel.html` showed explicit demonstration, demo source controls and simulated quotas, with real scenario absent. Preview covers compiled demo, not native real rows. | PASS within declared proof scope |

## Findings

Round-1 C1 stale-record diagnostic gap is resolved. No remaining correctness findings in the reviewed scope. The test now asserts both mismatched-birth and missing-process diagnostics; the independent release reproduction confirms the actual diagnostic path.

## Gates — verified at b7860f1

- `rtk proxy npm test`: 5 named tests passed, 0 failed.
- `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml`: 13 named tests passed, 0 failed, 0 ignored.
- `rtk npm run build`: typecheck and compiled build passed.
- `rtk npm run verify:native`: 20 named native assertions passed, 0 failed; pet, summary and panel ready; `error: null`, `uiErrors: []`.
- Rebuilt release `--discover-report`: nonexistent Claude PID omitted with explicit 1-discarded-record diagnostic; independent Codex provider remains present, unknown, with no request/command; Antigravity remains explicitly pending.
- Fault injection: not run, light profile.

## Limitations and workspace

Exact runtime polling cadence, external-agent lifecycle transitions, native real-mode visual inspection and physical gestures remain manual, as explicitly declared by C6/C7. No native real-screen visual pass is claimed. Smoke's unknown-session assertion can pass vacuously; independent local reports provide separate actual-session evidence. The smoke script regenerated tracked `.checks/native-smoke.json` (ready-array ordering only); verifier changed only this verification report directly. Diagnostic artifacts use unique ignored `/scratch/` directories and per-process source overrides without changing agent configuration. Confirmed zero Capy processes after verification.

