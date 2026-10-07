# Live task model verification — round 2

**Verdict: PASS for the scoped model switch and terminal-fit correction.**
**Profile:** light; no faults injected.
**Range:** `d401de9` plus reviewed working-tree changes to `src/style.css`, `src/tasks-ui.ts`, `scripts/verify-live-task-model.mjs` and shared native runner.
**Round:** 2, scoped; independent sub-agent, author != verifier.

The source/model assertions are carried from `.checks/live-task-model.verified.md` at `d401de9`; all proofs were rerun against the current desktop binary. The previous non-PASS presentation verdict was independently reexamined. This report does not close the entire multifunction checklist.

- `node scripts/verify-pet-native.mjs --live-task-model`: exit 0, 36 checks passed; `scratch/capy-visual-5f6393a4-e900-4843-9848-cb86b422f778/report.json`.
- `rtk npm test`: exit 0, all 32 named tests passed, none skipped, including initial-model exit confirmation and the external launcher test.

| Claim | Current evidence | Verdict |
| --- | --- | --- |
| Official selection before session-only `s` | `scripts/verify-live-task-model.mjs:14,19-21`; named native checks passed | PASS |
| Real Sonnet assistant receipt in original UUID | `scripts/verify-live-task-model.mjs:30,33`; receipt UUID `1f412f95-4a2a-4228-8679-96dbcdcaa201`, model `claude-sonnet-5-5`, marker `PROBE_71443110bbb3444d88fd25a90f8012af`; cwd is the own-task-project in this exact run | PASS |
| No tools; exact settings bytes | `scripts/verify-live-task-model.mjs:34,36`: rejects tool_use and uses Buffer.equals; corresponding checks passed | PASS |
| Initial-model labels in task title/list and exit | `scripts/verify-live-task-model.mjs:37`, `scripts/verify-pet-native.mjs:387`, `tests/exit-review.test.mjs:12`; actual native/UI checks and unit test passed | PASS |
| Renderer fits its visible terminal area | `scripts/verify-live-task-model.mjs:38`: `screen.top>=viewport.top && screen.left>=viewport.left && screen.bottom<=viewport.bottom && screen.right<=viewport.right`; new named native check passed | PASS |
| Window shows terminal without manual scrolling | Existing native assertion `scripts/verify-pet-native.mjs:165` passed on the inner viewport | PASS |

The fix moves the 10px padding/background to `.terminal-frame`; `#terminalViewport` is an unpadded 100%-sized child. This makes addon-fit's computed parent height the actual available terminal height. Independently inspected `model-switch-result.png`: final CLI status and bypass-permissions line are completely inside the dark frame, with space before the product hint. The prior visible overflow is resolved on the actual 760x680 native panel; the new child-rectangle assertion now tests the gap that the former parent-only assertion missed.

Limits: one observed panel size and existing Claude subscription profile were exercised. This proves neither all resize sizes nor other accounts/providers, quotas, API, or the full C16 objective. No product or runner edits by the verifier. Approved runner exit closed the own application and exact Claude process.
