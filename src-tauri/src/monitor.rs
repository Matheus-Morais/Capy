use crate::{discovery, DesktopState};
use std::time::Duration;
use tauri::{Emitter, Manager};

pub fn schedule(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        let state = app.state::<DesktopState>();
        let real = state
            .demo
            .lock()
            .map(|s| s.scenario == "real")
            .unwrap_or(false);
        if real {
            let mut report = discovery::scan(&discovery::Sources::local());
            let snapshot = (|| {
                let mut data = state.demo.lock().ok()?;
                if data.scenario != "real" {
                    return None;
                }
                state.preferences.lock().ok()?.apply(&mut report.sessions);
                if data.sessions == report.sessions && data.integrations == report.integrations {
                    return None;
                }
                data.sessions = report.sessions;
                data.integrations = report.integrations;
                Some(data.clone())
            })();
            if let Some(snapshot) = snapshot {
                if let Err(error) = app.emit("demo-updated", snapshot) {
                    eprintln!("Discovery update: {error}");
                }
            }
        }
        std::thread::sleep(Duration::from_secs(5));
    });
}
