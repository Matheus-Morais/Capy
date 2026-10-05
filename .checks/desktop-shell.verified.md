# Desktop shell Verification

**Verdict**: PASS
**Profile**: light
**Diff range**: `c74785dbf15d371c02f0cf11eb51203b6e1c0ccd` (full first commit; no prior baseline)
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier)

## Binding sources

| Source | Opened | Contradiction | Uncovered |
|---|---|---|---|
| `PRODUCT.md` | yes; Brand Commitments and Current Exploration | none; front pose is explicitly approved in Current Exploration | none for scoped surfaces |
| `prototypes/front-pet.svg` | yes; SVG root has `role="img"` and accessible label | none | none |
| `prototypes/preview.html` | yes; compact summary structure and simulated-data disclosure | none; native summary/panel screenshots retain session rows, quotas, actions and demo notice | none for scoped surfaces |

## Checks

| Check | Claim | Proof run | Evidence | Result |
|---|---|---|---|---|
| C1 | Build/typecheck pass and approved accessible frontal mascot renders | `rtk proxy npm.cmd run build` exit 0; screenshot | `src/pet.ts:2,8` imports/injects approved SVG; `index.html:3` button accessible name; `.impeccable/review/native-pet.png` frontal artwork | PASS |
| C2 | Only pet visible at startup with required native attributes and size | `npm test` (2 passed); `npm run verify:native` exit 0 | `tests/config.test.mjs:5-14` exact configuration assertions; `src-tauri/src/smoke.rs:30-48` runtime startup assertions; `.checks/native-smoke.json` startup_pet_only, frameless_topmost, logical_pet_size, tray_present all true. Shared `DesktopState` is registered on Builder before setup at `src-tauri/src/main.rs:274-280`; setup initializes `position_path` at `296-307`. | PASS |
| C3 | Summary toggles and full panel opens independently | `npm run verify:native` exit 0; screenshots | `.checks/native-smoke.json`: summary opens/hides, panel opens/hides true; `src-tauri/src/main.rs:147-163`; `summary.html:3` panel link; screenshots `.impeccable/review/native-summary.png`, `native-panel.png` | PASS |
| C4 | Drag threshold starts native drag once; click toggles; arrows move 16px | `rtk proxy npm.cmd run build` exit 0; located binding inspection | `src/pet.ts:26-42` primary pointer, >5px gate, one-shot `dragged`, click suppression and toggle; `44-47` arrow movement commands | PASS (native physical pointer gesture not automated; documented manual limitation) |
| C5 | Placement respects work area and stale position recovers | `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture` (6 passed); `npm run verify:native` exit 0 | `geometry::tests::clamp_with_negative_monitor_origin`, `stale_monitor_position_recovers_inside_primary`, `extreme_values_do_not_overflow` passed; `src-tauri/src/smoke.rs:82-124` runtime position bounds pass | PASS |
| C6 | Tray commands and close hide windows | `npm run verify:native` exit 0; located handler inspection | runtime `tray_present` true; `src-tauri/src/main.rs:246-263` show/hide/summary/panel/quit actions; `318-346` tray/menu wiring; `353-361` prevents close and hides | PASS |
| C7 | Normal exit saves position; malformed position falls back | `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture` (6 passed); `npm run verify:native` exit 0 | `position::tests::position_round_trip_and_invalid_json_fallback` passed; `src-tauri/src/smoke.rs:92-105` isolated save/restore assertions passed; `src-tauri/src/main.rs:353-361,376-378` close and process exit save | PASS |
| C8 | Demo responses update selected session; hide/restore affects followed count; demo indication visible | `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture` (6 passed); screenshots | `demo::tests::permission_and_question_continue_only_their_session` and `hide_restore_and_sleeping_change_followed_count` passed; `summary.html:3`, `panel.html:3` disclose simulated data; native screenshots show notice | PASS |

## Fault injection

Not run for the light profile. No faults were injected.

## Gate

- `rtk proxy npm.cmd run build` - exit 0.
- `npm test` - 2 passed, 0 failed (`companion window is transparent frameless topmost with a compact footprint`; `frontend capabilities do not expose shell or unrestricted filesystem`).
- `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture` - 6 passed, 0 failed (three geometry tests, two demo tests, position round trip/fallback).
- `npm run verify:native` - exit 0; `.checks/native-smoke.json` reports 17/17 assertions passed, pet/summary/panel ready, `uiErrors: []`.

No real integrations were exercised; sessions and quotas remain simulated as scoped. Native pointer gesture remains manual. Native smoke launched the release executable for its bounded self-test and exited it; it did not leave Capy running.


## Concrete assertions and handler conditions

| Check | Concrete assertion/condition | Located evidence |
|---|---|---|
| C1 | Production command completes successfully; `artwork` imports `../prototypes/front-pet.svg?raw` and is assigned to `#petArtwork`; the button accessible name is `Capy: abrir ou fechar resumo`. | `src/pet.ts:2,8`; `index.html:3`; `prototypes/front-pet.svg:1` |
| C2 | Config test asserts transparent/decorations=false/alwaysOnTop/skipTaskbar true and width=200, height=180; smoke asserts pet visible AND summary/panel hidden, plus native undecorated/topmost and scaled logical size within 1 px. | `tests/config.test.mjs:5-14`; `src-tauri/src/smoke.rs:30-48` |
| C3 | First `toggle_summary` call must leave `summary.is_visible()` true; second must leave it false; panel path must make `panel.is_visible()` true. | `src-tauri/src/smoke.rs:49-72`; command condition `src-tauri/src/main.rs:147-163` |
| C4 | Pointer handler returns at distance `<= 5` or after `pointer.dragged`; beyond threshold it sets `dragged=true` before calling `startDragging()`; click is suppressed for that drag, otherwise calls `toggle_summary`; arrow map sends ±16. | `src/pet.ts:31-47` |
| C5 | Geometry tests assert clamped x/y and restored point lie within work area, including a negative-origin area and stale monitor fallback; native smoke tests current point equals its clamped point. | `src-tauri/src/geometry.rs:72-126`; `src-tauri/src/smoke.rs:107-124` |
| C6 | Tray lookup `app.tray_by_id("capy").is_some()`; menu IDs dispatch `show`, `hide`, `summary`, `panel`, `quit`; close request calls `prevent_close()` then `hide_window(...)`. | `src-tauri/src/smoke.rs:48`; `src-tauri/src/main.rs:246-263,311-346,353-361` |
| C7 | Test asserts `load(save(point)) == Some(point)` and invalid JSON returns `None`; smoke asserts loaded saved point equals `point` and restored native x/y equal that point. | `src-tauri/src/position.rs:21-37`; `src-tauri/src/smoke.rs:92-105` |
| C8 | Tests assert answering permission/question changes only the selected session to working; hide then restore changes followed count; UI labels mark summary and panel data as simulated. | `src-tauri/src/demo.rs:106-139`; `summary.html:3`; `panel.html:3` |
