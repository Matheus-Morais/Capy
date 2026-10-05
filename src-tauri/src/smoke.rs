use crate::{geometry::Point, position};

fn wait_ready(app: &tauri::AppHandle, label: &str) -> bool {
    for _ in 0..50 {
        if app
            .state::<crate::DesktopState>()
            .ready
            .lock()
            .map(|r| r.contains(label))
            .unwrap_or(false)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}
use std::{path::PathBuf, time::Duration};
use tauri::{Manager, PhysicalPosition};

pub fn schedule(app: tauri::AppHandle, report_path: PathBuf) {
    std::thread::spawn(move || {
        let pet_ready = wait_ready(&app, "pet");
        let mut checks = Vec::<(&str, bool)>::new();
        let result = (|| -> Result<(), String> {
            checks.push(("frontend_pet_ready", pet_ready));
            let pet = crate::window(&app, "pet")?;
            let summary = crate::window(&app, "summary")?;
            let panel = crate::window(&app, "panel")?;
            checks.push((
                "startup_pet_only",
                pet.is_visible().map_err(|e| e.to_string())?
                    && !summary.is_visible().map_err(|e| e.to_string())?
                    && !panel.is_visible().map_err(|e| e.to_string())?,
            ));
            checks.push((
                "frameless_topmost",
                !pet.is_decorated().map_err(|e| e.to_string())?
                    && pet.is_always_on_top().map_err(|e| e.to_string())?,
            ));
            let size = pet.inner_size().map_err(|e| e.to_string())?;
            let scale = pet.scale_factor().map_err(|e| e.to_string())?;
            checks.push((
                "logical_pet_size",
                (f64::from(size.width) / scale - 200.0).abs() < 1.0
                    && (f64::from(size.height) / scale - 180.0).abs() < 1.0,
            ));
            checks.push(("tray_present", app.tray_by_id("capy").is_some()));
            crate::toggle_summary(app.clone())?;
            checks.push(("frontend_summary_ready", wait_ready(&app, "summary")));
            checks.push((
                "summary_opens",
                summary.is_visible().map_err(|e| e.to_string())?,
            ));
            crate::toggle_summary(app.clone())?;
            checks.push((
                "summary_hides",
                !summary.is_visible().map_err(|e| e.to_string())?,
            ));
            crate::show_panel(app.clone())?;
            checks.push(("frontend_panel_ready", wait_ready(&app, "panel")));
            checks.push((
                "frontend_ipc_ready_all_three_windows",
                app.state::<crate::DesktopState>()
                    .ready
                    .lock()
                    .map(|r| r.len() == 3)
                    .unwrap_or(false),
            ));
            checks.push((
                "panel_opens",
                panel.is_visible().map_err(|e| e.to_string())?,
            ));
            crate::hide_window(app.clone(), "panel".into())?;
            checks.push((
                "panel_hides",
                !panel.is_visible().map_err(|e| e.to_string())?,
            ));
            crate::hide_window(app.clone(), "pet".into())?;
            crate::show_pet(&app)?;
            checks.push(("pet_restores", pet.is_visible().map_err(|e| e.to_string())?));
            let area = crate::work_area(&pet)?;
            let size = pet.outer_size().map_err(|e| e.to_string())?;
            let point = area.clamp(
                Point {
                    x: area.x.saturating_add(100),
                    y: area.y.saturating_add(100),
                },
                size.width,
                size.height,
            );
            pet.set_position(PhysicalPosition::new(point.x, point.y))
                .map_err(|e| e.to_string())?;
            crate::persist_pet(&app)?;
            let state = app.state::<crate::DesktopState>();
            let path = state
                .position_path
                .get()
                .ok_or("Caminho de posição indisponível")?;
            checks.push(("position_saved", position::load(path) == Some(point)));
            crate::restore_pet(&app)?;
            let restored = pet.outer_position().map_err(|e| e.to_string())?;
            checks.push((
                "position_restored",
                restored.x == point.x && restored.y == point.y,
            ));
            crate::move_pet(app.clone(), 16, 0)?;
            let current = pet.outer_position().map_err(|e| e.to_string())?;
            checks.push((
                "position_in_work_area",
                area.contains(Point {
                    x: current.x,
                    y: current.y,
                }) && area.clamp(
                    Point {
                        x: current.x,
                        y: current.y,
                    },
                    size.width,
                    size.height,
                ) == Point {
                    x: current.x,
                    y: current.y,
                },
            ));
            checks.push((
                "unknown_window_rejected",
                crate::hide_window(app.clone(), "unknown".into()).is_err(),
            ));
            crate::demo_action(
                app.clone(),
                app.state::<crate::DesktopState>(),
                "scenario".into(),
                "".into(),
                "real".into(),
            )?;
            for _ in 0..70 {
                if app
                    .state::<crate::DesktopState>()
                    .demo
                    .lock()
                    .map(|s| s.integrations.len() == 3)
                    .unwrap_or(false)
                {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            let snapshot = crate::demo_snapshot(app.state::<crate::DesktopState>())?;
            checks.push((
                "real_discovery_poll_ready",
                snapshot.scenario == "real" && snapshot.integrations.len() == 3,
            ));
            checks.push((
                "real_sessions_unknown",
                snapshot
                    .sessions
                    .iter()
                    .all(|s| s.state == "unknown" && s.request.is_none()),
            ));
            checks.push((
                "real_responses_rejected",
                crate::demo_action(
                    app.clone(),
                    app.state::<crate::DesktopState>(),
                    "allow".into(),
                    "any".into(),
                    "".into(),
                )
                .is_err(),
            ));
            Ok(())
        })();
        let passed = result.is_ok() && checks.iter().all(|(_, passed)| *passed);
        let state = app.state::<crate::DesktopState>();
        let ready = state
            .ready
            .lock()
            .map(|r| r.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let ui_errors = state
            .ui_errors
            .lock()
            .map(|r| r.clone())
            .unwrap_or_default();
        let passed = passed && ui_errors.is_empty();
        let report = serde_json::json!({ "passed": passed, "checks": checks.iter().map(|(name, passed)| serde_json::json!({"name":name, "passed":passed})).collect::<Vec<_>>(), "error":result.err(), "ready":ready, "uiErrors":ui_errors });
        if let Some(parent) = report_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let wrote =
            std::fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).is_ok();
        app.exit(if passed && wrote { 0 } else { 1 });
    });
}
