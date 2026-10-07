#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activity;
mod api_accounts;
mod credential_vault;
mod chat_api;
mod chat_history;
mod chat_cli;
mod chat_cli_presence;
mod chat_process;
mod chat_recovery;
mod chat_commands;
mod chat_presence;
mod chat_transfer;
mod exit_review;
mod antigravity;
mod claude_activity;
mod demo;
mod discovery;
mod geometry;
mod interventions;
mod monitor;
mod position;
mod quotas;
mod quota_policy;
mod settings;
mod profiles;
mod claude_quotas;
mod terminal;
mod tasks;
mod routing;
mod handoff;
mod handoff_context;
mod loaded_instructions;
mod smoke;
mod source_access;

use geometry::{Area, Point};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
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
    app.emit("pet-visibility",true).map_err(|e|e.to_string())?;
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
    panel.set_focus().map_err(|e| e.to_string())?;
    app.emit("attention-viewed",()).map_err(|e|e.to_string())
}
#[tauri::command]
fn hide_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if !["pet", "summary", "panel"].contains(&label.as_str()) {
        return Err("Janela desconhecida".into());
    }
    window(&app, &label)?.hide().map_err(|e| e.to_string())?;
    if label=="pet"{app.emit("pet-visibility",false).map_err(|e|e.to_string())?;}
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
fn save_preferences(app: tauri::AppHandle, value: settings::Preferences) -> Result<(), String> {
    app.state::<settings::Store>().save(value.clone())?;
    let snapshot = {
        let state = app.state::<DesktopState>();
        let mut data = state.demo.lock().map_err(|_| "Estado indisponível.")?;
        data.reduce_motion = value.reduce_motion;
        data.preferences = value;
        data.clone()
    };
    app.emit("demo-updated", snapshot).map_err(|e| e.to_string())
}
#[tauri::command]
fn dismiss_quota_alert(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let service = app.state::<quota_policy::Service>();
    service.dismiss(&id)?;
    let snapshot = {
        let state = app.state::<DesktopState>();
        let mut data = state.demo.lock().map_err(|_| "Estado indisponível.")?;
        data.quota_alerts = service.snapshot();
        data.clone()
    };
    app.emit("demo-updated", snapshot).map_err(|e| e.to_string())
}
#[tauri::command]
fn list_profiles(store: State<'_, profiles::Store>) -> Result<Vec<profiles::Profile>, String> { store.list() }
#[tauri::command]
fn add_profile(store: State<'_, profiles::Store>, label:String, existing:Option<PathBuf>, billing:String) -> Result<profiles::Profile,String> {
    store.add(label,existing,billing)
}
#[tauri::command]
async fn profile_identity(app:tauri::AppHandle,id:String,cwd:Option<PathBuf>)->Result<profiles::Identity,String>{
    let profile=app.state::<profiles::Store>().get(&id)?;
    if cwd.as_ref().is_some_and(|p|!p.is_absolute()||!p.is_dir()){return Err("Pasta de trabalho inválida.".into());}
    tauri::async_runtime::spawn_blocking(move||app.state::<profiles::Store>().probe(&profile,cwd.as_deref())).await.map_err(|_|"Vínculo com o CLI indisponível.".to_owned())?
}
#[tauri::command]
fn login_profile(store:State<'_,profiles::Store>,id:String)->Result<(),String>{profiles::login(&store.get(&id)?,&store.root)}
#[tauri::command]
fn connect_claude_quotas(store:State<'_,profiles::Store>,id:String,enabled:bool)->Result<(),String>{claude_quotas::configure(&store.get(&id)?,enabled)}
#[tauri::command]
fn list_tasks(store:State<'_,Arc<tasks::Store>>)->Result<Vec<tasks::Task>,String>{store.list()}
#[tauri::command]
async fn start_task(app:tauri::AppHandle,request:tasks::Start)->Result<tasks::Task,String>{
    let profile=app.state::<profiles::Store>().get(&request.profile_id)?;
    let store=app.state::<Arc<tasks::Store>>().inner().clone();
    let terminal=app.state::<Arc<terminal::Service>>().inner().clone();
    let output_app=app.clone();let exit_app=app.clone();
    tauri::async_runtime::spawn_blocking(move||{
        let task=store.prepare(&profile,request)?;
        let id=task.id.clone();
        let exit=app.state::<exit_review::Service>();let _admission=exit.admit()?;
        store.launch(task,&profile,&terminal,move|chunk|{let _=output_app.emit("terminal-output",chunk);},move||{let _=exit_app.emit("terminal-exited",&id);})
    }).await.map_err(|_|"Não foi possível preparar a tarefa.".to_owned())?
}
#[tauri::command]
fn terminal_replay(service:State<'_,Arc<terminal::Service>>,id:String)->Result<terminal::Replay,String>{service.replay(&id)}
#[tauri::command]
fn terminal_input(service:State<'_,Arc<terminal::Service>>,id:String,data:String)->Result<(),String>{service.input(&id,&data)}
#[tauri::command]
fn terminal_resize(service:State<'_,Arc<terminal::Service>>,id:String,cols:u16,rows:u16)->Result<(),String>{service.resize(&id,cols,rows)}
#[tauri::command]
fn terminal_interrupt(service:State<'_,Arc<terminal::Service>>,id:String,confirmed:bool)->Result<(),String>{
    if !confirmed{return Err("Confirme a interrupção da sessão selecionada.".into());}service.input(&id,"\u{1b}")
}
#[tauri::command]
fn terminal_model_picker(service:State<'_,Arc<terminal::Service>>,id:String)->Result<(),String>{service.input(&id,"\u{1b}p")}
fn exit_resources(app:&tauri::AppHandle)->Result<Vec<exit_review::Resource>,String>{
    let active=app.state::<Arc<terminal::Service>>().active_ids()?;
    let tasks=if active.is_empty(){Vec::new()}else{app.state::<Arc<tasks::Store>>().list()?};
    let chats=app.state::<Arc<chat_history::Store>>().list().map_err(|error|format!("Histórico incompatível ou indisponível. A saída foi bloqueada; os arquivos foram preservados. {error}"))?;
    exit_review::resources(chats,tasks,active)
}
#[tauri::command]
fn prepare_exit_review(app:tauri::AppHandle)->Result<exit_review::Review,String>{
    app.state::<exit_review::Service>().prepare(quotas::now_ms(),||exit_resources(&app))
}
#[tauri::command]
fn cancel_exit_review(app:tauri::AppHandle,nonce:String)->Result<(),String>{app.state::<exit_review::Service>().cancel(&nonce)}
#[tauri::command]
fn confirm_exit(app:tauri::AppHandle,nonce:String,confirmed:bool)->Result<(),String>{
    app.state::<exit_review::Service>().approve(&nonce,confirmed,quotas::now_ms(),||exit_resources(&app))?;
    if let Err(error)=app.state::<Arc<terminal::Service>>().close_all(){
        app.state::<exit_review::Service>().reopen_after_close_failure()?;return Err(error);
    }
    if let Err(error)=persist_pet(&app){eprintln!("Position save: {error}");}
    app.exit(0);Ok(())
}
#[tauri::command]
fn request_exit(app:tauri::AppHandle)->Result<(),String>{
    let review=match prepare_exit_review(app.clone()){
        Ok(review)=>review,
        Err(error)=>{show_panel(app.clone())?;app.emit("exit-review-error",&error).map_err(|emit|emit.to_string())?;return Err(error);}
    };
    if review.resources.is_empty(){return confirm_exit(app,review.nonce,true);}
    show_panel(app.clone())?;
    app.emit("confirm-exit",review).map_err(|error|error.to_string())
}
#[tauri::command]
fn list_handoffs(store:State<'_,Arc<handoff::Store>>)->Result<Vec<handoff::Review>,String>{store.list()}
#[tauri::command]
async fn prepare_handoff(app:tauri::AppHandle,source_id:String,destination_id:String,model:String)->Result<handoff::Review,String>{
    let task=app.state::<Arc<tasks::Store>>().get(&source_id)?;
    let source=app.state::<profiles::Store>().get(&task.profile_id)?;
    let destination=app.state::<profiles::Store>().get(&destination_id)?;
    let store=app.state::<Arc<handoff::Store>>().inner().clone();
    tauri::async_runtime::spawn_blocking(move||store.prepare(&task,&source,&destination,model,false)).await.map_err(|_|"Não foi possível preparar a continuação.".to_owned())?
}
#[tauri::command]
fn cancel_handoff(store:State<'_,Arc<handoff::Store>>,nonce:String)->Result<(),String>{store.cancel(&nonce)}
#[tauri::command]
async fn approve_handoff(app:tauri::AppHandle,nonce:String,summary:handoff::Summary,billing_confirmed:bool,mode:String)->Result<tasks::Task,String>{
    let store=app.state::<Arc<handoff::Store>>().inner().clone();
    let mut review=store.list()?.into_iter().find(|r|r.nonce==nonce).ok_or("Esta revisão já foi usada ou cancelada.")?;
    let task=app.state::<Arc<tasks::Store>>().get(&review.source_task_id)?;
    let source=app.state::<profiles::Store>().get(&task.profile_id)?;
    let destination=app.state::<profiles::Store>().get(&review.destination_profile_id)?;
    let task_store=app.state::<Arc<tasks::Store>>().inner().clone();
    let terminal=app.state::<Arc<terminal::Service>>().inner().clone();
    let output_app=app.clone();let exit_app=app.clone();
    tauri::async_runtime::spawn_blocking(move||{
        let exit=app.state::<exit_review::Service>();let _admission=exit.admit()?;
        review.summary=summary.clone();
        let request=store.approve(&nonce,summary,billing_confirmed,&task,&source,&destination,mode)?;
        let result=(||{
            let task=task_store.prepare(&destination,request)?;let id=task.id.clone();
            if !claude_activity::boundary_unchanged(&source.config_dir,&review.source_task_id,&task.cwd.to_string_lossy(),&review.boundary){
                return Err("A sessão de origem mudou durante a verificação da conta. Aguarde o novo fim de turno e revise novamente.".into());
            }
            task_store.launch(task,&destination,&terminal,move|chunk|{let _=output_app.emit("terminal-output",chunk);},move||{let _=exit_app.emit("terminal-exited",&id);})
        })();
        if result.is_err(){store.restore_after_failure(review)?;}result
    }).await.map_err(|_|"Não foi possível transferir a tarefa.".to_owned())?
}
#[tauri::command]
async fn resume_task(app:tauri::AppHandle,id:String)->Result<(),String>{
    let store=app.state::<Arc<tasks::Store>>().inner().clone();
    let task=store.get(&id)?;let profile=app.state::<profiles::Store>().get(&task.profile_id)?;
    tauri::async_runtime::spawn_blocking(move||{
        let sources=discovery::Sources{claude:profile.config_dir.clone(),codex:PathBuf::new(),antigravity:PathBuf::new()};
        if discovery::scan(&sources).sessions.iter().any(|s|s.id==format!("claude:{}",task.id)) {return Err("Esta conversa já está aberta. Use o terminal integrado ou o terminal original.".into());}
        let identity=profiles::identity(&profile,Some(&task.cwd))?;
        if identity.account!=task.account||identity.billing!=task.billing{return Err("A conta ou cobrança mudou. Verifique antes de continuar.".into());}
        let exit=app.state::<exit_review::Service>();let _admission=exit.admit()?;
        store.resume_external(&task,&profile)
    }).await.map_err(|_|"Não foi possível abrir a conversa.".to_owned())?
}
fn intervention_source(
    app: &tauri::AppHandle,
    id: &str,
) -> Result<interventions::registry::Source, String> {
    let state = app.state::<DesktopState>();
    let data = state.demo.lock().map_err(|_| "Estado indisponível.")?;
    let session = data
        .sessions
        .iter()
        .find(|s| s.id == id)
        .ok_or("Sessão não encontrada. Atualize a lista.")?;
    interventions::registry::Source::from_session(session, data.scenario == "real")
        .map_err(|_| "Esta sessão não está disponível para respostas reais.".into())
}
#[tauri::command]
async fn connect_interventions(
    app: tauri::AppHandle,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let source = intervention_source(&app, &id)?;
    let service = app
        .state::<Arc<interventions::service::Service>>()
        .inner()
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        if enabled {
            service.connect(source)
        } else {
            service.disconnect(source.thread_id)
        }
    })
    .await
    .map_err(|_| "Serviço de respostas indisponível.".to_owned())?
}
#[tauri::command]
async fn respond_intervention(
    app: tauri::AppHandle,
    context: interventions::registry::Context,
    response: serde_json::Value,
) -> Result<(), String> {
    let source = intervention_source(&app, &context.session_id)?;
    let service = app
        .state::<Arc<interventions::service::Service>>()
        .inner()
        .clone();
    tauri::async_runtime::spawn_blocking(move || service.respond(context, source, response))
        .await
        .map_err(|_| "A entrega não foi confirmada. Confira na origem.".to_owned())?
}
#[tauri::command]
async fn open_source(app:tauri::AppHandle,state: State<'_, DesktopState>, id: String) -> Result<(), String> {
    let expected = {
        let snapshot = state.demo.lock().map_err(|_| "Estado indisponível")?;
        if snapshot.scenario != "real" {
            return Err("A demonstração não abre sessões reais.".into());
        }
        snapshot
            .sessions
            .iter()
            .find(|s| s.id == id)
            .cloned()
            .ok_or("Sessão não encontrada. Atualize a lista.")?
    };
    if expected.kind=="chat" {
        if !state.ready.lock().map_err(|_|"Estado do painel indisponível.")?.contains("panel"){
            return Err("O painel ainda está carregando. Tente abrir a conversa novamente em um instante.".into());
        }
        let chat_id=expected.id.strip_prefix("chat:").ok_or("Identidade de conversa inválida.")?;
        let chat=app.state::<Arc<chat_history::Store>>().get(chat_id)?;
        if chat.title!=expected.project||chat.target.provider!=expected.agent{return Err("A conversa mudou; atualize a lista.".into());}
        show_panel(app.clone())?;
        app.get_webview_window("panel").ok_or("Painel indisponível.")?.emit("chat-selected",&chat.id).map_err(|_|"Não foi possível selecionar a conversa.".to_owned())?;
        return Ok(());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let current = discovery::scan(&discovery::Sources::local());
        source_access::open(&expected, true, &current)
    })
    .await
    .map_err(|_| "Não foi possível verificar a sessão de origem.")?
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
    if action == "motion" {
        if !["true", "false"].contains(&answer.as_str()) { return Err("Preferência inválida.".into()); }
        let mut prefs = app.state::<settings::Store>().get()?;
        prefs.reduce_motion = answer == "true";
        return save_preferences(app, prefs);
    }
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
        if action == "scenario" && answer != "real" {
            app.state::<Arc<interventions::service::Service>>().reset();
        }
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
        "quit" => request_exit(app.clone()),
        _ => Err("Ação desconhecida".into()),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(dir) = args.iter().position(|a| a == "--claude-statusline").and_then(|i| args.get(i+1)) {
        let _ = claude_quotas::hook(PathBuf::from(dir));
        return;
    }
    if let Some(session_id) = args
        .iter()
        .position(|a| a == "--intervention-probe")
        .and_then(|i| args.get(i + 1))
    {
        #[cfg(feature = "intervention-proof")]
        if let Err(error) = intervention_probe(session_id) {
            eprintln!("Intervention probe: {error}");
            std::process::exit(1);
        }
        #[cfg(not(feature = "intervention-proof"))]
        {
            let _ = session_id;
            eprintln!("Intervention probe is only available in the isolated proof build.");
            std::process::exit(1);
        }
        return;
    }
    if let Some(event) = args
        .iter()
        .position(|a| a == "--antigravity-hook")
        .and_then(|i| args.get(i + 1))
    {
        antigravity::collect(event);
        return;
    }
    if args.iter().any(|a| a == "--claude-hook") {
        let lease=args.iter().position(|a|a=="--chat-lease").and_then(|i|args.get(i+1)).map(PathBuf::from);
        claude_activity::collect(lease.as_deref());
        return;
    }
    if let Some(path) = args
        .iter()
        .position(|a| a == "--quota-report")
        .and_then(|i| args.get(i + 1))
    {
        let rows = quotas::Cache::default().poll(&discovery::Sources::local());
        if serde_json::to_vec_pretty(&rows)
            .map_err(|e| e.to_string())
            .and_then(|bytes| std::fs::write(path, bytes).map_err(|e| e.to_string()))
            .is_err()
        {
            std::process::exit(1);
        }
        return;
    }
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
    let visual_root=args.iter().position(|a|a=="--visual-test").and_then(|i|args.get(i+1)).map(|arg|{
        let root=PathBuf::from(arg).canonicalize().expect("A pasta de prova visual deve existir.");
        let scratch=PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().expect("Workspace ausente").join("scratch").canonicalize().expect("Scratch ausente");
        assert!(root.starts_with(scratch)&&root.file_name().and_then(|n|n.to_str()).is_some_and(|n|n.starts_with("capy-visual-")),"A prova visual deve ficar em scratch/capy-visual-*");
        root
    });
    let test_mode = smoke_path.is_some()||visual_root.is_some();
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
        .manage(exit_review::Service::default())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Err(e) = show_pet(app) {
                eprintln!("Show Capy: {e}");
            }
        }))
        .invoke_handler(tauri::generate_handler![
            chat_commands::list_api_accounts,
            chat_commands::add_api_account,
            chat_commands::list_chats,
            chat_commands::prepare_chat_recovery,
            chat_commands::approve_chat_recovery,
            chat_commands::create_chat,
            chat_commands::send_chat,
            chat_commands::chat_transfer_review,
            chat_commands::prepare_chat_transfer,
            chat_commands::approve_chat_transfer,
            chat_commands::cancel_chat_transfer,
            toggle_summary,
            show_panel,
            hide_window,
            move_pet,
            demo_snapshot,
            demo_action,
            save_preferences,
            dismiss_quota_alert,
            list_profiles,
            add_profile,
            profile_identity,
            login_profile,
            connect_claude_quotas,
            list_tasks,
            start_task,
            terminal_replay,
            terminal_input,
            terminal_resize,
            terminal_interrupt,
            terminal_model_picker,
            resume_task,
            confirm_exit,
            prepare_exit_review,
            cancel_exit_review,
            request_exit,
            list_handoffs,
            prepare_handoff,
            approve_handoff,
            cancel_handoff,
            open_source,
            connect_interventions,
            respond_intervention,
            ui_ready,
            ui_error
        ])
        .setup(move |app| {
            let position_path = if let Some(root)=&visual_root{
                root.join("position.json")
            }else if test_mode {
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
            let settings = settings::Store::load(position_path.with_file_name("preferences.json"));
            let prefs = settings.get()?;
            {
                let state = app.state::<DesktopState>();
                let mut snapshot = state.demo.lock().map_err(|_| "Estado indisponível.")?;
                snapshot.reduce_motion = prefs.reduce_motion;
                snapshot.preferences = prefs;
            }
            app.manage(settings);
            app.manage(Arc::new(api_accounts::Store::load(position_path.with_file_name("api-accounts.json"))));
            let chats=chat_history::Store::load(position_path.with_file_name("chat"));
            if let Err(error)=chats.recover_interrupted(){
                app.state::<DesktopState>().ui_errors.lock().map_err(|_|"Erros indisponíveis.")?.push(format!("Histórico de chat: {error}"));
            }
            app.manage(Arc::new(chats));
            app.manage(chat_presence::Service::default());
            app.manage(profiles::Store::load(position_path.parent().ok_or("Pasta de configuração indisponível.")?.to_path_buf()));
            app.manage(Arc::new(tasks::Store::load(position_path.parent().ok_or("Pasta de configuração indisponível.")?.to_path_buf())));
            app.manage(Arc::new(terminal::Service::default()));
            app.manage(Arc::new(handoff::Store::load(position_path.with_file_name("handoffs.json"))));
            app.manage(quota_policy::Service::load(position_path.with_file_name("quota-alerts.json")));
            *app.state::<DesktopState>()
                .preferences
                .lock()
                .map_err(|_| "Preferências indisponíveis")? =
                discovery::Preferences::load(position_path.with_file_name("hidden-sessions.json"));
            let guard_app = app.handle().clone();
            let service = interventions::service::Service::new(
                discovery::Sources::local().codex,
                move |expected| {
                    let state = guard_app.state::<DesktopState>();
                    if !state.demo.lock().is_ok_and(|data| data.scenario == "real") {
                        return false;
                    }
                    let mut report = discovery::scan(&discovery::Sources::local());
                    let Ok(preferences) = state.preferences.lock() else {
                        return false;
                    };
                    preferences.apply(&mut report.sessions);
                    drop(preferences);
                    let valid = report.sessions.iter().any(|session| {
                        interventions::registry::Source::from_session(session, true).as_ref()
                            == Ok(expected)
                    });
                    valid && state.demo.lock().is_ok_and(|data| data.scenario == "real")
                },
            );
            app.manage(Arc::new(service));
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
                app.state::<Arc<interventions::service::Service>>().stop();
                if let Err(e) = persist_pet(app) {
                    eprintln!("Position save: {e}");
                }
            }
        });
}

#[cfg(feature = "intervention-proof")]
fn intervention_probe(session_id: &str) -> Result<(), String> {
    use std::io::{BufRead, Write};
    let sources = discovery::Sources::local();
    let proof_root = std::env::var_os("CAPY_INTERVENTION_PROBE_ROOT")
        .map(PathBuf::from)
        .ok_or("A origem de teste isolada não foi informada.")?
        .canonicalize()
        .map_err(|_| "A origem de teste não existe.")?;
    let scratch = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Diretório do repositório indisponível.")?
        .join("scratch")
        .canonicalize()
        .map_err(|_| "Diretório de prova indisponível.")?;
    if !proof_root.starts_with(&scratch)
        || !proof_root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("codex-capy-"))
        || !proof_root.join("AGENTS.md").is_file()
    {
        return Err(
            "A prova aceita somente uma pasta própria dentro de scratch/codex-capy-*".into(),
        );
    }
    let thread_id = session_id
        .strip_prefix("codex:")
        .filter(|thread| discovery::uuid(thread))
        .ok_or("ID de conversa de prova inválido.")?;
    let source = interventions::registry::Source {
        session_id: session_id.to_owned(),
        thread_id: thread_id.to_owned(),
        origin: "isolated-proof".into(),
    };
    let expected = source.clone();
    let thread_to_close = expected.thread_id.clone();
    let valid_source = expected.clone();
    let validate = move |actual: &interventions::registry::Source| *actual == valid_source;
    let service = interventions::service::Service::new(sources.codex, validate);
    service.connect(source.clone())?;
    let (input, receiver) = std::sync::mpsc::sync_channel::<String>(8);
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if input.send(line).is_err() {
                break;
            }
        }
    });
    let mut emitted = HashSet::new();
    let start = std::time::Instant::now();
    println!(
        "{{\"event\":\"connected\",\"sessionId\":{}}}",
        serde_json::to_string(session_id).unwrap_or_default()
    );
    let _ = std::io::stdout().flush();
    loop {
        if start.elapsed() >= std::time::Duration::from_secs(300) {
            return Err("Tempo de prova excedido.".into());
        }
        let (requests, _) = service.snapshot();
        for request in requests {
            if request.status == "pending" && emitted.insert(request.context.nonce.clone()) {
                println!(
                    "{{\"event\":\"request\",\"request\":{}}}",
                    serde_json::to_string(&request).map_err(|e| e.to_string())?
                );
                let _ = std::io::stdout().flush();
            }
        }
        match receiver.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(line) => {
                let command: serde_json::Value =
                    serde_json::from_str(&line).map_err(|_| "Comando de prova inválido.")?;
                let context: interventions::registry::Context =
                    serde_json::from_value(command["context"].clone())
                        .map_err(|_| "Contexto de prova inválido.")?;
                match service.respond(context, expected.clone(), command["response"].clone()) {
                    Ok(()) => println!("{{\"event\":\"resolved\"}}"),
                    Err(error) => println!(
                        "{{\"event\":\"rejected\",\"message\":{}}}",
                        serde_json::to_string(&error).map_err(|e| e.to_string())?
                    ),
                }
                let _ = std::io::stdout().flush();
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    let _ = service.disconnect(thread_to_close);
    Ok(())
}
