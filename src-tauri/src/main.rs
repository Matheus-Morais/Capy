#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activity;
mod demo;
mod discovery;
mod geometry;
mod monitor;
mod position;
mod smoke;

use geometry::{Area, Point};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow};

struct DesktopState {
    demo: Mutex<demo::Snapshot>,
    preferences: Mutex<discovery::Preferences>,
    position_path: OnceLock<PathBuf>,
    ready: Mutex<HashSet<String>>,
    ui_errors: Mutex<Vec<String>>,
}

fn window(app: &tauri::AppHandle, label: &str) -> Result<WebviewWindow, String> {
    app.get_webview_window(label)
        .ok_or_else(|| format!("Janela {label} não está disponível"))
}
fn area(monitor: &tauri::Monitor) -> Area {
    let work = monitor.work_area();
    Area {
        x: work.position.x,
        y: work.position.y,
        width: work.size.width,
        height: work.size.height,
    }
}
fn work_area(pet: &WebviewWindow) -> Result<Area, String> {
    pet.current_monitor()
        .map_err(|e| e.to_string())?
        .or(pet.primary_monitor().map_err(|e| e.to_string())?)
        .as_ref()
        .map(area)
        .ok_or("Nenhum monitor disponível".into())
}
fn restore_pet(app: &tauri::AppHandle) -> Result<(), String> {
    let pet = window(app, "pet")?;
    let primary = pet
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("Nenhum monitor disponível")?;
    let monitors = pet.available_monitors().map_err(|e| e.to_string())?;
    let areas: Vec<Area> = monitors.iter().map(area).collect();
    let size = pet.outer_size().map_err(|e| e.to_string())?;
    let state = app.state::<DesktopState>();
    let path = state.position_path.get().ok_or("Capy está iniciando")?;
    let saved = position::load(path);
    let point = geometry::restored_position(saved, &areas, area(&primary), size.width, size.height);
    pet.set_position(PhysicalPosition::new(point.x, point.y))
        .map_err(|e| e.to_string())?;
    pet.show().map_err(|e| e.to_string())
}
fn persist_pet(app: &tauri::AppHandle) -> Result<(), String> {
    let position = window(app, "pet")?
        .outer_position()
        .map_err(|e| e.to_string())?;
    let state = app.state::<DesktopState>();
    let path = state.position_path.get().ok_or("Capy está iniciando")?;
    position::save(
        path,
        Point {
            x: position.x,
            y: position.y,
        },
    )
    .map_err(|e| e.to_string())
}
fn position_summary(app: &tauri::AppHandle) -> Result<(), String> {
    let pet = window(app, "pet")?;
    let summary = window(app, "summary")?;
    let area = work_area(&pet)?;
    let scale = pet.scale_factor().map_err(|e| e.to_string())?;
    let width = ((380.0 * scale).round() as u32).min(area.width);
    let height = ((620.0 * scale).round() as u32).min(area.height);
    summary
        .set_size(PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    let pet_point = pet.outer_position().map_err(|e| e.to_string())?;
    let pet_size = pet.outer_size().map_err(|e| e.to_string())?;
    let point = area.clamp(
        Point {
            x: pet_point
                .x
                .saturating_add(pet_size.width as i32)
                .saturating_sub(width as i32),
            y: pet_point.y.saturating_sub(height as i32).saturating_sub(8),
        },
        width,
        height,
    );
    summary
        .set_position(PhysicalPosition::new(point.x, point.y))
        .map_err(|e| e.to_string())
}
fn show_pet(app: &tauri::AppHandle) -> Result<(), String> {
    let pet = window(app, "pet")?;
    let point = pet.outer_position().map_err(|e| e.to_string())?;
    let size = pet.outer_size().map_err(|e| e.to_string())?;
    let primary = pet
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("Nenhum monitor disponível")?;
    let areas: Vec<Area> = pet
        .available_monitors()
        .map_err(|e| e.to_string())?
        .iter()
        .map(area)
        .collect();
    let clamped = geometry::restored_position(
        Some(Point {
            x: point.x,
            y: point.y,
        }),
        &areas,
        area(&primary),
        size.width,
        size.height,
    );
    pet.set_position(PhysicalPosition::new(clamped.x, clamped.y))
        .map_err(|e| e.to_string())?;
    pet.show().map_err(|e| e.to_string())?;
    pet.set_focus().map_err(|e| e.to_string())
}
fn show_summary(app: &tauri::AppHandle) -> Result<(), String> {
    let pet = window(app, "pet")?;
    if !pet.is_visible().map_err(|e| e.to_string())? {
        show_pet(app)?;
    }
    position_summary(app)?;
    let summary = window(app, "summary")?;
    summary.show().map_err(|e| e.to_string())?;
    summary.set_focus().map_err(|e| e.to_string())?;
    app.emit("summary-visibility", true)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn toggle_summary(app: tauri::AppHandle) -> Result<(), String> {
    if window(&app, "summary")?
        .is_visible()
        .map_err(|e| e.to_string())?
    {
        hide_window(app, "summary".into())
    } else {
        show_summary(&app)
    }
}
#[tauri::command]
fn show_panel(app: tauri::AppHandle) -> Result<(), String> {
    let panel = window(&app, "panel")?;
    panel.center().map_err(|e| e.to_string())?;
    panel.show().map_err(|e| e.to_string())?;
    panel.set_focus().map_err(|e| e.to_string())
}
#[tauri::command]
fn hide_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if !["pet", "summary", "panel"].contains(&label.as_str()) {
        return Err("Janela desconhecida".into());
    }
    window(&app, &label)?.hide().map_err(|e| e.to_string())?;
    if label == "summary" || label == "pet" {
        window(&app, "summary")?.hide().map_err(|e| e.to_string())?;
        app.emit("summary-visibility", false)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
fn move_pet(app: tauri::AppHandle, dx: i32, dy: i32) -> Result<(), String> {
    if dx.abs_diff(0) > 16 || dy.abs_diff(0) > 16 {
        return Err("Deslocamento inválido".into());
    }
    let pet = window(&app, "pet")?;
    let current = pet.outer_position().map_err(|e| e.to_string())?;
    let scale = pet.scale_factor().map_err(|e| e.to_string())?;
    let size = pet.outer_size().map_err(|e| e.to_string())?;
    let point = work_area(&pet)?.clamp(
        Point {
            x: current
                .x
                .saturating_add((f64::from(dx) * scale).round() as i32),
            y: current
                .y
                .saturating_add((f64::from(dy) * scale).round() as i32),
        },
        size.width,
        size.height,
    );
    pet.set_position(PhysicalPosition::new(point.x, point.y))
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn demo_snapshot(state: State<'_, DesktopState>) -> Result<demo::Snapshot, String> {
    state
        .demo
        .lock()
        .map(|s| s.clone())
        .map_err(|_| "Estado indisponível".into())
}
#[tauri::command]
fn ui_ready(window: WebviewWindow, state: State<'_, DesktopState>) -> Result<(), String> {
    state
        .ready
        .lock()
        .map_err(|_| "Estado indisponível")?
        .insert(window.label().into());
    Ok(())
}
#[tauri::command]
fn ui_error(window: WebviewWindow, state: State<'_, DesktopState>, message: String) {
    if let Ok(mut errors) = state.ui_errors.lock() {
        if errors.len() < 20 {
            errors.push(format!(
                "{}: {}",
                window.label(),
                message.chars().take(500).collect::<String>()
            ));
        }
    }
}
#[tauri::command]
fn demo_action(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    action: String,
    id: String,
    answer: String,
) -> Result<(), String> {
    let snapshot = {
        let mut data = state.demo.lock().map_err(|_| "Estado indisponível")?;
        if data.scenario == "real" && ["hide", "restore"].contains(&action.as_str()) {
            let mut preferences = state
                .preferences
                .lock()
                .map_err(|_| "Preferências indisponíveis")?;
            let mut updated = preferences.clone();
            if action == "hide" {
                if !data.sessions.iter().any(|s| s.id == id) {
                    return Err("Sessão não encontrada".into());
                }
                updated.hidden.insert(id.clone());
            } else {
                updated.hidden.clear();
            }
            updated.save()?;
            *preferences = updated;
        }
        data.apply(&action, &id, &answer)?;
        data.clone()
    };
    app.emit("demo-updated", snapshot)
        .map_err(|e| e.to_string())
}
fn tray_action(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    match id {
        "show" => show_pet(app),
        "hide" => {
            persist_pet(app)?;
            hide_window(app.clone(), "pet".into())
        }
        "summary" => show_summary(app),
        "panel" => show_panel(app.clone()),
        "quit" => {
            if let Err(e) = persist_pet(app) {
                eprintln!("Position save: {e}");
            }
            app.exit(0);
            Ok(())
        }
        _ => Err("Ação desconhecida".into()),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(path) = args
        .iter()
        .position(|a| a == "--discover-report" || a == "--activity-report")
        .and_then(|i| args.get(i + 1))
    {
        let sources = discovery::Sources::local();
        let mut report = discovery::scan(&sources);
        if args.iter().any(|a| a == "--activity-report") {
            activity::enrich(&sources, &mut report);
        }
        let result = serde_json::to_vec_pretty(&report)
            .map_err(|e| e.to_string())
            .and_then(|bytes| std::fs::write(path, bytes).map_err(|e| e.to_string()));
        if let Err(error) = result {
            eprintln!("Discovery report: {error}");
            std::process::exit(1);
        }
        return;
    }
    let smoke_path = args
        .iter()
        .position(|a| a == "--self-test")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);
    let test_mode = smoke_path.is_some();
    let builder = tauri::Builder::default()
        .manage(DesktopState {
            demo: Mutex::new(if test_mode {
                demo::Snapshot::default()
            } else {
                demo::Snapshot::real()
            }),
            preferences: Mutex::new(discovery::Preferences::default()),
            position_path: OnceLock::new(),
            ready: Mutex::new(HashSet::new()),
            ui_errors: Mutex::new(Vec::new()),
        })
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Err(e) = show_pet(app) {
                eprintln!("Show Capy: {e}");
            }
        }))
        .invoke_handler(tauri::generate_handler![
            toggle_summary,
            show_panel,
            hide_window,
            move_pet,
            demo_snapshot,
            demo_action,
            ui_ready,
            ui_error
        ])
        .setup(move |app| {
            let position_path = if test_mode {
                std::env::temp_dir()
                    .join(format!("capy-smoke-{}", std::process::id()))
                    .join("position.json")
            } else {
                app.path().app_config_dir()?.join("position.json")
            };
            app.state::<DesktopState>()
                .position_path
                .set(position_path.clone())
                .map_err(|_| "Caminho de posição já inicializado")?;
            *app.state::<DesktopState>()
                .preferences
                .lock()
                .map_err(|_| "Preferências indisponíveis")? =
                discovery::Preferences::load(position_path.with_file_name("hidden-sessions.json"));
            monitor::schedule(app.handle().clone());
            let menu = Menu::with_items(
                app,
                &[
                    &MenuItem::with_id(app, "show", "Mostrar Capy", true, None::<&str>)?,
                    &MenuItem::with_id(app, "hide", "Ocultar Capy", true, None::<&str>)?,
                    &MenuItem::with_id(app, "summary", "Resumo", true, None::<&str>)?,
                    &MenuItem::with_id(app, "panel", "Painel completo", true, None::<&str>)?,
                    &MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?,
                ],
            )?;
            TrayIconBuilder::with_id("capy")
                .icon(
                    app.default_window_icon()
                        .ok_or("Ícone indisponível")?
                        .clone(),
                )
                .tooltip("Capy — seu companheiro de trabalho")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if let Err(e) = tray_action(app, event.id.as_ref()) {
                        eprintln!("Tray action: {e}");
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        if let Err(e) = show_summary(tray.app_handle()) {
                            eprintln!("Open summary: {e}");
                        }
                    }
                })
                .build(app)?;
            restore_pet(app.handle())?;
            if let Some(path) = smoke_path.clone() {
                smoke::schedule(app.handle().clone(), path);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if let Err(e) = persist_pet(window.app_handle()) {
                    eprintln!("Position save: {e}");
                }
                if let Err(e) = hide_window(window.app_handle().clone(), window.label().into()) {
                    eprintln!("Hide window: {e}");
                }
            }
            tauri::WindowEvent::Moved(_) if window.label() == "pet" => {
                if let Some(summary) = window.app_handle().get_webview_window("summary") {
                    if summary.is_visible().unwrap_or(false) {
                        let _ = position_summary(window.app_handle());
                    }
                }
            }
            _ => {}
        });
    builder
        .build(tauri::generate_context!())
        .expect("Não foi possível iniciar o Capy")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                if let Err(e) = persist_pet(app) {
                    eprintln!("Position save: {e}");
                }
            }
        });
}
