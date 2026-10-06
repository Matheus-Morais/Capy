use crate::{activity, discovery, DesktopState};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

pub fn schedule(app: tauri::AppHandle) {
    let quota_rows = Arc::new(Mutex::new(Vec::new()));
    let quota_output = quota_rows.clone();
    let quota_app = app.clone();
    std::thread::spawn(move || {
        let mut cache = crate::quotas::Cache::default();
        loop {
            let started = Instant::now();
            let real = quota_app
                .state::<DesktopState>()
                .demo
                .lock()
                .map(|s| s.scenario == "real")
                .unwrap_or(false);
            if real {
                let rows = cache.poll(&discovery::Sources::local());
                if let Ok(mut output) = quota_output.lock() {
                    *output = rows;
                }
            }
            std::thread::sleep(Duration::from_secs(5).saturating_sub(started.elapsed()));
        }
    });
    std::thread::spawn(move || loop {
        let started = Instant::now();
        let state = app.state::<DesktopState>();
        let real = state
            .demo
            .lock()
            .map(|s| s.scenario == "real")
            .unwrap_or(false);
        if real {
            let sources = discovery::Sources::local();
            let mut report = discovery::scan(&sources);
            activity::enrich(&sources, &mut report);
            let quotas = quota_rows
                .lock()
                .map(|rows| rows.clone())
                .unwrap_or_default();
            let snapshot = (|| {
                let mut data = state.demo.lock().ok()?;
                if data.scenario != "real" {
                    return None;
                }
                state.preferences.lock().ok()?.apply(&mut report.sessions);
                if data.sessions == report.sessions
                    && data.integrations == report.integrations
                    && data.quotas == quotas
                {
                    return None;
                }
                data.sessions = report.sessions;
                data.integrations = report.integrations;
                data.quotas = quotas;
                Some(data.clone())
            })();
            if let Some(snapshot) = snapshot {
                if let Err(error) = app.emit("demo-updated", snapshot) {
                    eprintln!("Discovery update: {error}");
                }
            }
        }
        std::thread::sleep(Duration::from_secs(5).saturating_sub(started.elapsed()));
    });
}
