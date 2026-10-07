# Live task model verification

**Verdict: FAIL for terminal presentation; PASS for the scoped model-switch behavior.**
**Profile:** light; no faults injected. No Coverage or Test policy sections were declared.
**Diff range:** `a4b3189..d401de9`.
**Round:** 1, additional scoped review; not the whole multifunction goal.
**Verifier:** independent sub-agent, author != verifier.

Read `.checks/multifunction.md` C16 and its two new Landing/proof paragraphs, `.design/multifunction.md`, the range diff, and TLC Verify instructions. The approved behavior permits an official same-session switch. The observed choice is consistent with it. Official [Claude Code model configuration](https://code.claude.com/docs/en/model-config), opened on 2026-10-07, documents picker `s` as session-only and Enter/direct `/model <name>` as saving a default.

## Independently executed evidence

- `node scripts/verify-pet-native.mjs --live-task-model`: exit 0, all 35 named checks passed; artifacts `scratch/capy-visual-76b982e1-c0b7-40b3-b1db-f371b978464b/report.json`.
- `rtk npm test`: exit 0, 31 named tests passed, none skipped. Includes `exit confirmation names exact resources and distinguishes uncertain consumption`.
- Inspected `model-sonnet-selected.png`, `model-switch-result.png`, `model-picker-0.txt`, `task-initial-geometry.json`, and `model-switch-receipt.json` from this independent run.

## Scoped checks

| Claim | Located assertion and observed evidence | Result |
| --- | --- | --- |
| Sonnet explicitly selected before committing | `scripts/verify-live-task-model.mjs:14,19-21`: selected-row regex must match Sonnet, `check(...,selected)` precedes `terminal_input` with `data:'s'`; screenshot shows arrow on Sonnet 5.5, checkmark still on Haiku 4.5 | PASS |
| Official session-only switch | `scripts/verify-live-task-model.mjs:21` sends `s`, matching official documentation; picker then closes | PASS |
| Actual assistant response in exact original UUID | `scripts/verify-live-task-model.mjs:30,33`: `row.type==='assistant' && row.sessionId===task.id && ...includes('sonnet') && ...part.text.includes(marker)`; receipt has UUID `97e7aaf7-f497-44d4-af06-39af6b50f82a`, model `claude-sonnet-5-5`, assistant marker `PROBE_b72e9ccf80ab49be8e45a23cd6175a04` | PASS |
| Preserve exact folder/context, no response inferred from echo | Initial receipt proof `scripts/verify-pet-native.mjs:174,178` asserts assistant receipt, original user cwd/instruction and Haiku. The subsequent Sonnet receipt independently contains the same original cwd; its type and message role are assistant, not user or terminal text | PASS |
| No tools executed | `scripts/verify-live-task-model.mjs:34`: rejects any assistant `tool_use` in the exact task history | PASS |
| Exact settings bytes retained | `scripts/verify-live-task-model.mjs:8,35-36`: before/after Buffers compared with `before.equals(after)`, also handles missing-file preservation. Native named check passed, hash `8e4a54a472e2ec1019db410fe1ef4d68d3c8b569ec2347c503a31d7647bb0825` recorded | PASS |
| Title/list/exit say initial model | `scripts/verify-live-task-model.mjs:37` checks title and managed task list for `modelo inicial: haiku`; `scripts/verify-pet-native.mjs:387` checks actual exit dialog; `tests/exit-review.test.mjs:12` asserts same literal. Screenshot title agrees while CLI identifies Sonnet | PASS |
| Responsive terminal height fits content | `scripts/verify-pet-native.mjs:165` checks the parent box only. Independent screenshot has terminal footer spilling outside the dark parent; no child-containment assertion exists | FAIL |

## Concrete presentation finding

`src/style.css:153` gives `#terminalViewport` height `min(70vh,600px)` and parent padding 10px with global border-box sizing. In the captured 760x680 panel the parent rectangle is top 169.3125, bottom 645.3125, height 476. The final CLI status/bypass-permissions footer extends below the dark rectangle, to approximately y654, directly above the product hint. The selected-model screenshot likewise places the final picker row partly below that rectangle. This is visible overflow, not merely a preferred spacing change.

Installed `node_modules/@xterm/addon-fit/src/FitAddon.ts:72-88` computes available height from the parent computed height and subtracts padding from the xterm element, not the parent's padding. That explains the visible mismatch with parent padding; the fit needs to account for its actual available content box. Native assertion at line 165 cannot detect it because it measures only `#terminalViewport` against the outer window. Add a child screen/content containment check alongside the fix, retaining the existing parent/window check.

Limits: native runner completed before a supplemental live child-geometry query could attach, so overflow is supported by the actual screenshot and recorded parent geometry rather than a measured child rectangle. Only the existing Claude subscription profile, same-account Haiku-to-Sonnet integrated terminal was exercised. Other accounts/providers, external CLI, quotas, API, and the complete C16/multifunction goal remain outside this scoped verdict. No product or runner changes were made by this verifier. The runner approved exit and confirmed closure of its own application and exact Claude process; the native app is free for the next proof.
