use crate::{activity, discovery, DesktopState};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

pub fn schedule(app: tauri::AppHandle) {
    let agy_rows=Arc::new(Mutex::new(crate::antigravity_quotas::cached_rows()));
    let agy_output=agy_rows.clone();let agy_app=app.clone();
    std::thread::spawn(move||{
        let mut cache=crate::antigravity_quotas::Cache::default();let mut revision=0;
        loop {
            let started=Instant::now();
            if agy_app.state::<DesktopState>().demo.lock().map(|s|s.scenario=="real").unwrap_or(false) {
                let next=agy_app.state::<crate::quota_refresh::Service>().revision();
                let rows=cache.poll(next!=revision);revision=next;
                if let Ok(mut output)=agy_output.lock(){*output=rows;}
            }
            std::thread::sleep(Duration::from_secs(5).saturating_sub(started.elapsed()));
        }
    });
    let quota_rows = Arc::new(Mutex::new(Vec::new()));
    let quota_output = quota_rows.clone();
    let quota_app = app.clone();
    std::thread::spawn(move || {
        let mut cache = crate::quotas::Cache::default();
        let mut claude_rows = Vec::new();
        let mut last_claude = Instant::now() - Duration::from_secs(60);
        let mut profile_signature = String::new();
        loop {
            let started = Instant::now();
            let real = quota_app
                .state::<DesktopState>()
                .demo
                .lock()
                .map(|s| s.scenario == "real")
                .unwrap_or(false);
            if real {
                if quota_app.state::<crate::quota_refresh::Service>().take() {
                    cache.refresh();last_claude=Instant::now()-Duration::from_secs(60);
                }
                let mut rows = cache.poll(&discovery::Sources::local());
                rows.retain(|row| row.provider == "Codex");
                let profiles = quota_app.state::<crate::profiles::Store>().list().unwrap_or_default();
                let signature = serde_json::to_string(&profiles).unwrap_or_default();
                if last_claude.elapsed() >= Duration::from_secs(60) || profile_signature != signature {
                    claude_rows = profiles.iter().flat_map(crate::claude_quotas::poll).collect();
                    let prefs=quota_app.state::<crate::settings::Store>().get().unwrap_or_default();
                    let chats=quota_app.state::<Arc<crate::chat_history::Store>>().list().unwrap_or_default();
                    for profile in profiles.iter().filter(|profile|prefs.quota_rules.iter().any(|rule|
                        rule.fallback.iter().any(|d|d.profile_id==profile.id)||chats.iter().any(|chat|
                            chat.target.profile_id==profile.id&&chat.target.provider==rule.provider&&chat.target.account==rule.account))){
                        let _=quota_app.state::<crate::profiles::Store>().probe(profile,None);
                    }
                    last_claude = Instant::now();
                    profile_signature = signature;
                }
                rows.extend(claude_rows.clone());
                rows.extend(agy_rows.lock().map(|rows|rows.clone()).unwrap_or_default());
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
            crate::profiles::enrich_sessions(&app.state::<crate::profiles::Store>(), &sources, &mut report);
            let quotas = quota_rows
                .lock()
                .map(|rows| rows.clone())
                .unwrap_or_default();
            let prefs = app.state::<crate::settings::Store>().get().unwrap_or_default();
            let alerts = app.state::<crate::quota_policy::Service>()
                .evaluate(&quotas, &prefs, crate::quotas::now_ms())
                .unwrap_or_else(|error| { eprintln!("Quota alerts: {error}"); app.state::<crate::quota_policy::Service>().snapshot() });
            let profile_store=app.state::<crate::profiles::Store>();
            let profile_rows=profile_store.list();
            let handoff_store=app.state::<Arc<crate::handoff::Store>>();
            let now=crate::quotas::now_ms();
            let candidates=profile_store.candidates(now);
            let mut routing=Vec::new();
            let tasks=app.state::<Arc<crate::tasks::Store>>().list();
            let chat_rows=app.state::<Arc<crate::chat_history::Store>>().list();
            if let (Ok(task_rows),Ok(profiles),Ok(chats))=(&tasks,&profile_rows,&chat_rows){
                let mut retained=task_rows.iter().map(|task|task.id.clone()).collect::<Vec<_>>();
                retained.extend(chats.iter().filter(|chat|chat.target.kind=="claudeCli").map(|chat|chat.id.clone()));
                retained.extend(report.sessions.iter().filter(|session|session.kind=="claude").filter_map(|session|session.id.strip_prefix("claude:").filter(|id|discovery::uuid(id)).map(str::to_owned)));
                for profile in profiles{let _=crate::loaded_instructions::prune(&profile.config_dir,&retained);}
            }
            if let Ok(tasks)=&tasks{let _=handoff_store.prune(tasks);}
            for task in tasks.unwrap_or_default().into_iter().rev().take(64){
                if !handoff_store.eligible(&task){continue;}
                let Some(account)=task.account.as_deref()else{continue;};
                let Some(rule)=prefs.quota_rules.iter().find(|r|r.provider=="Claude"&&r.account==account)else{continue;};
                let Ok(source)=profile_store.get(&task.profile_id)else{continue;};
                let boundary=crate::claude_activity::turn_boundary(&source.config_dir,&task.id,&task.cwd.to_string_lossy(),now);
                if boundary.as_ref().is_some_and(|b|!handoff_store.should_prepare(&task,b)){continue;}
                let decision=crate::routing::decide(rule,&task.profile_id,&task.model,&quotas,&candidates,boundary.is_some(),now);
                match decision {
                crate::routing::Decision::Review{destination}=>{
                    if let Ok(profile)=profile_store.get(&destination.profile_id){
                        if let Err(error)=handoff_store.prepare(&task,&source,&profile,destination.model,true){routing.push(crate::routing::Status{task_id:task.id.clone(),state:"unavailable".into(),message:error});}
                    }
                },
                crate::routing::Decision::WaitingTurn=>routing.push(crate::routing::Status{task_id:task.id,state:"waitingTurn".into(),message:"Percentual de troca atingido. Aguardando confirmação do fim do turno atual.".into()}),
                crate::routing::Decision::Exhausted=>routing.push(crate::routing::Status{task_id:task.id,state:"exhausted".into(),message:"Nenhuma alternativa disponível na cadeia. Verifique o login e as quotas das contas de destino.".into()}),
                crate::routing::Decision::Idle=>{}
                };
            }
            let handoffs=handoff_store.list().unwrap_or_default();
            if let Ok(profiles)=&profile_rows{
                let mut chat_targets=profile_store.chat_targets(now);
                if let Ok(accounts)=app.state::<Arc<crate::api_accounts::Store>>().list(){
                    chat_targets.extend(accounts.into_iter().filter(|a|a.configured).map(|a|crate::chat_history::Target{
                        kind:"api".into(),profile_id:a.account.id,provider:a.account.provider,account:a.account.label,
                        billing:"api".into(),credential_revision:Some(a.account.revision),
                    }));
                }
                match crate::chat_routing::evaluate(&app.state::<Arc<crate::chat_history::Store>>(),profiles,&chat_targets,&quotas,&prefs,now){
                    Ok(statuses)=>routing.extend(statuses),
                    Err(error)=>routing.push(crate::routing::Status{task_id:"chat".into(),state:"unavailable".into(),message:error}),
                }
            }
            let (mut interventions, mut subscriptions) = app
                .state::<Arc<crate::interventions::service::Service>>()
                .snapshot();
            let _ = (|| {
                let mut data = state.demo.lock().ok()?;
                if data.scenario != "real" {
                    return None;
                }
                let chat_transfers=app.state::<Arc<crate::chat_history::Store>>().transfer_reviews().unwrap_or_default();
                if let Ok(chats)=&chat_rows{
                    report.sessions.extend(app.state::<crate::chat_presence::Service>().rows(&chats,crate::quotas::now_ms()));
                }
                state.preferences.lock().ok()?.apply(&mut report.sessions);
                interventions.retain(|v| {
                    report
                        .sessions
                        .iter()
                        .any(|s| s.id == v.context.session_id && !s.hidden)
                });
                subscriptions.retain(|v| {
                    report
                        .sessions
                        .iter()
                        .any(|s| s.id == v.session_id && !s.hidden)
                });
                if data.sessions == report.sessions
                    && data.integrations == report.integrations
                    && data.quotas == quotas
                    && data.interventions == interventions
                    && data.subscriptions == subscriptions
                    && data.quota_alerts == alerts
                    && serde_json::to_value(&data.handoffs).ok()==serde_json::to_value(&handoffs).ok()
                    && data.routing==routing
                    && serde_json::to_value(&data.chat_transfers).ok()==serde_json::to_value(&chat_transfers).ok()
                {
                    return None;
                }
                data.sessions = report.sessions;
                data.integrations = report.integrations;
                data.quotas = quotas;
                data.interventions = interventions;
                data.subscriptions = subscriptions;
                data.quota_alerts = alerts;
                data.handoffs = handoffs;
                data.routing = routing;
                data.chat_transfers=chat_transfers;
                if let Err(error) = app.emit("demo-updated", data.clone()) {
                    eprintln!("Discovery update: {error}");
                }
                Some(())
            })();
        }
        std::thread::sleep(Duration::from_secs(5).saturating_sub(started.elapsed()));
    });
}
