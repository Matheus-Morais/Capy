# Token-Watch quota verification

**Verdict: PARTIAL for the complete user request; PASS for all 11 local extension checklist obligations.**
**Profile:** light (no root AGENTS.md declares another profile; supplied RTK instructions applied).
**Diff range:** `d843983..220014037ce6ca1b29a10dbf030708566e99f0c1`.
**Round:** 2, final source; independent verifier, author != verifier.

The binding source `.design/token-watch-quota.md` and complete `.checks/token-watch-quota.md` were opened. They distinguish local delivery from acceptance with real accounts and providers. No contradiction found. No implementation edits, provider sends, live flags, global credential/settings changes, Token-Watch changes, or production app restarts were performed by this verifier. Only this report is a verifier-authored artifact.

## Checks

| # | Obligation | Independent evidence at final source | Result |
|---|---|---|---|
| 1 | Separate accounts and windows | `tests/quota-board.test.mjs:23`: `assert.equal((result.match(/class="quota-account"/g)??[]).length,2)`; line 24 checks both window labels. Lines 38-39 reject executable labels and assert escaping. Supplemental direct Node assertions checked two providers with the same identity produce two accounts and three bucket regions. Source `src/quota-board.ts:54,63` keys provider/account and bucket separately. | PASS |
| 2 | Available balance, window, relative/exact renewal | Test lines 24-25 assert `aria-valuenow="24"`, five-hour/weekly labels and `2h 0min`; lines 42-43 assert `1d 1h` and `1min`. Supplemental direct Node assertion required `Renovação: ` plus the exact `pt-BR` short date/time calculated from the fixture reset; source `src/quota-board.ts:40`. | PASS |
| 3 | No expired/invalid/unidentified real percentages | Test lines 27-34 run stale state, TTL+1, future observation, missing identity, elapsed reset, NaN, 101%; line 32 asserts no meter/value/percentage markup; TTL exactly 120000 remains valid. `src/quota-board.ts:5-12` also rejects invalid duration/reset/date bounds. | PASS |
| 4 | Reevaluate locally without backend event | Test lines 52-54 first requires a meter, advances `Date.now` to TTL+1 and invokes timer, then requires no meter and `Dado expirado`; `cleared===true` proves cancellation. Supplemental direct Node assertions advanced 60 seconds, required `1h 59min`, and required preserved open disclosure/focus. Source `src/quota-board.ts:74-82` preserves keyed focus and schedules minute/TTL/reset deadlines. | PASS |
| 5 | Prominent panel and compact summary | Independent `npm run build` passed. Native runner `scripts/verify-pet-native.mjs:170-172` asserts board precedes session grid and 3 demo accounts/6 meters; lines 190-191 assert compact 540px real panel has no horizontal overflow; lines 492-494 assert summary no overflow and 3 accounts/6 meters. Final captures inspected: `scratch/capy-visual-48d99913-aae1-4401-a770-cfc579441fc7/summary.png`; earlier compact real capture also inspected. Functional hierarchy/grouping/windows verified; binding source does not specify exact spacing/type/color or a different meter shape. | PASS |
| 6 | Accounts, alerts, continuity navigation/focus | Runner lines 173-178 clicks all three shortcuts, asserts focused controls within accounts/preferences and expanded chat details plus focused form. All ran and passed. `src/dashboard.ts:135-152` opens ancestors and focuses target; summary opens full panel. | PASS |
| 7 | Four Antigravity windows, identity, original time, disabled != zero | Cargo tests `antigravity_quotas.rs:69-71` assert 4 rows, exact account/75% consumed, 2 weekly windows; line 75 asserts old/future observed time is retained with no windows; line 82 asserts disabled/invalid/reset-expired rows have no window; lines 86-87 assert missing/65537-byte file unavailable. Read path lines 20-27 uses original file mtime, bounded read and unchanged metadata checks; parse enumerates Gemini/Other five-hour/weekly windows at lines 35-36. Final actual snapshot retains 4 rows, 3 stale + 1 disabled, original timestamp 1791472870737 and no balances. This proves stale/disabled handling, not fresh actual Antigravity acceptance. | PASS |
| 8 | Background refresh, max one request/15s, no rejuvenation | `quota_refresh.rs:18-19`: first request succeeds, immediate second errors, pending take is single-use. Line 8 enforces `Duration::from_secs(15)`. Native runner line 194 rejects repeated command with `15 segundos`. `monitor.rs:26-45` refreshes cache and repolls Claude; `claude_quotas.rs:129-131` loads/sorts existing observation timestamps rather than writing new ones; AGY reader preserves mtime. | PASS |
| 9 | Claude CLI/API chain, end-of-turn review, no send/billing before approval | Cargo `chat_routing.rs:74-78` asserts working/unknown wait, no review, and completed revision selection; lines 98-111 iterate OpenAI/Anthropic/Gemini, assert exact automatic destination, unchanged one-conversation/two-message source, rejected missing billing consent, and rejected credential revision drift; lines 115-118 assert unconfigured API exhausted/no review. Monitor lines 114-125 adds only configured API targets. `chat_commands.rs:155-164` verifies both identities and calls guarded approval before transport execution; `chat_transfer.rs:161-172` pins fresh review nonce/source revision/identity/billing/single use. Final native runner lines 217-232 asserts exact destination/model, required unchecked review+billing consent, retained edited summary, cancellation tombstone/no destination/no active send, and rejected canceled approval. Native synthetic proof does not exercise a provider send. | PASS |
| 10 | Missing saved destination never silently substituted | Test lines 12-13 requires saved disabled selected option and rejects other account selected. This test was added by the feature and ran. | PASS |
| 11 | Real alerts/routing never labeled simulation | Final test lines 17-18 asserts real account/routing visible in real mode and empty markup for all four demo scenarios. Native runner line 197 asserts hidden/empty `#quotaAlerts` after real->waiting. Final summary capture contains no leaked real alert. | PASS |

## Binding UI enumeration

| Surface | Functional decisions in binding source | Coverage |
|---|---|---|
| Panel | Quotas before sessions; provider/account groups; windows/available balance/countdown; stale state; account/alert/chat shortcuts; subscription vs API disclosure | Tests 1-6,9,11 above plus native panel ordering/counts/focus and real-mode unavailable snapshot. No uncovered functional composition found. |
| Compact summary | Same quota information in compact width; full-panel access | Native summary counts/overflow, source shared renderer, inspected final summary image. |
| Transfer review | Exact account/AI destination, summary review and changed billing consent before send | Native synthetic API review fields/consent/cancel proof and Rust source/approval guards. |

No `Coverage`, `Test policy`, or `Swept` table exists in this checklist. Their joins/row verdicts are absent inputs, not silently assumed passes. Fault injection is not required under light profile and was not performed. UI structural obligations were nevertheless audited against the binding source.

## Complete-request acceptance still open

| Requested real acceptance | Evidence and limit | Result |
|---|---|---|
| Fresh real Codex quota | Earlier independent run at 447c8ca (`scratch/capy-visual-78c85f08-4479-49e4-b2c7-17da16359807`) observed actual identified Codex 44%/64% remaining and its compact capture matched. Final 2200140 snapshot has Codex unavailable, so final native meter-equality predicate runs on zero fresh meters. It must not be counted as fresh final real balance proof. | PARTIAL |
| Real Claude official statusline for connected accounts | Final snapshot: account null, unavailable, requests connecting quotas. Unit fixtures prove parsing/identity/TTL, not the real statusline connection. | UNPROVEN |
| Fresh actual Antigravity sample | Existing actual identified file is expired; original time is preserved; one window disabled. No recent sample available in this proof. | UNPROVEN |
| Alerts on alternate real accounts | Unit crossing/reset/account-isolation tests run; final UI prevents real-alert leakage into demo. No alternate-account real threshold crossing was produced. | UNPROVEN |
| Real account/AI continuity and approved API sends | Local ordered routing and approval guards pass; native review is an explicitly synthetic OpenAI target canceled before send. No real alternate Claude account or OpenAI/Anthropic/Gemini provider call was made. | UNPROVEN |

At 447c8ca an actual Codex alert appeared beneath a simulation caption in the verifier's own summary image. This defect was independently reported by the finish reviewer and fixed in 2200140. It is closed by the final 43-test gate, native real->demo check and recaptured summary. Other real acceptance gaps remain open; this report does not declare the full product/request complete.

## Gates independently run

- Final `rtk proxy npm test`: 43 passed, 0 failed (all named quota proofs appeared).
- Final `rtk proxy npm run build`: TypeScript check and Vite build passed.
- Final `rtk proxy cargo test`: 140 passed, 0 failed, 2 intentionally ignored live subscription tests. Named Antigravity/routing/refresh tests appeared. Existing five warnings remain, with no new gate failure.
- Final `node scripts/verify-pet-native.mjs --quota-dashboard`: 36 passed; report/captures/snapshot at `scratch/capy-visual-48d99913-aae1-4401-a770-cfc579441fc7`.
- Final `node scripts/verify-pet-native.mjs --automatic-chat-api-review`: 37 passed; report/capture at `scratch/capy-visual-3467016f-059e-4af4-80cd-6220e8222216`.
- Supplemental ephemeral Node assertions independently passed exact renewal, provider/bucket separation, minute countdown and disclosure focus preservation; they did not modify source or persist fixtures.

All own native runners completed with their cleanup paths. Other workspace changes to `.specs/STATE.md` and `NEXT_STEPS.md` belonged to the orchestrator and were preserved.
