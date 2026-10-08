use crate::{chat_history::{self,Store,Target},profiles::Profile,quotas::Row,routing::{self,Decision,Status},settings::Preferences};

pub fn evaluate(store:&Store,profiles:&[Profile],targets:&[Target],quotas:&[Row],prefs:&Preferences,now:u64)->Result<Vec<Status>,String>{
    let candidates=targets.iter().filter(|target|chat_history::valid_target(target)).map(|target|routing::Candidate{
        profile_id:target.profile_id.clone(),account:Some(target.account.clone()),provider:target.provider.clone(),ready:true,
    }).collect::<Vec<_>>();
    let mut statuses=Vec::new();
    for source in store.list()?{
        if source.target.kind!="claudeCli"||source.target.billing!="subscription"||source.messages.is_empty()||source.transferred_to.is_some(){continue;}
        let Some(rule)=prefs.quota_rules.iter().find(|rule|rule.provider==source.target.provider&&rule.account==source.target.account)else{continue;};
        let ended=!["working","unknown"].contains(&source.state.as_str())&&source.active_nonce.is_none();
        let decision=routing::decide(rule,&source.target.profile_id,&source.model,quotas,&candidates,ended,now);
        if decision==Decision::Idle{continue;}
        let id=format!("chat:{}",source.id);
        if !targets.iter().any(|target|target==&source.target&&chat_history::valid_target(target)){
            statuses.push(Status{task_id:id,state:"unavailable".into(),message:"A identidade recente da conta deste chat não foi confirmada. Verifique o perfil antes de preparar a troca.".into()});continue;
        }
        match decision{
            Decision::WaitingTurn=>statuses.push(Status{task_id:id,state:"waitingTurn".into(),message:"Percentual de troca atingido. Aguardando o fim confirmado do envio deste chat.".into()}),
            Decision::Exhausted=>statuses.push(Status{task_id:id,state:"exhausted".into(),message:"Nenhuma alternativa disponível na cadeia deste chat. Verifique o login e as quotas das contas de destino.".into()}),
            Decision::Review{destination}=>{
                let Some(target)=targets.iter().find(|target|target.profile_id==destination.profile_id&&chat_history::valid_target(target))else{continue;};
                let result=(||{
                    let profile=profiles.iter().find(|profile|profile.id==source.target.profile_id).ok_or("Perfil de origem indisponível.")?;
                    let workspace=store.workspace(&source.id)?;
                    let guides=crate::loaded_instructions::render(&profile.config_dir,&source.id,&workspace)
                        .unwrap_or_else(|error|Some(format!("Não foi possível ler referências capturadas: {error}")));
                    store.prepare_automatic_transfer(&source,target.clone(),destination.model,guides.as_deref())
                })();
                if let Err(error)=result{statuses.push(Status{task_id:id,state:"unavailable".into(),message:error});}
            }
            Decision::Idle=>{}
        }
    }
    Ok(statuses)
}

#[cfg(test)]
mod tests{
    use super::*;
    struct Fixture{root:std::path::PathBuf,store:Store,profiles:Vec<Profile>,targets:Vec<Target>,prefs:Preferences,source:chat_history::Conversation}
    impl Fixture{
        fn new()->Self{
            let root=std::env::temp_dir().join(format!("capy-chat-routing-{}",uuid::Uuid::new_v4()));let store=Store::load(root.join("chat"));
            let targets=[("p","source@example.invalid"),("q","next@example.invalid"),("r","last@example.invalid")].into_iter().map(|(id,account)|Target{
                kind:"claudeCli".into(),profile_id:id.into(),provider:"Claude".into(),account:account.into(),billing:"subscription".into(),credential_revision:None,
            }).collect::<Vec<_>>();
            let profiles=targets.iter().map(|target|Profile{id:target.profile_id.clone(),label:target.account.clone(),provider:"Claude".into(),config_dir:root.join(&target.profile_id),billing:"subscription".into()}).collect();
            let source=store.create("Own quota fixture".into(),targets[0].clone(),"haiku".into()).unwrap();
            let nonce=uuid::Uuid::new_v4().to_string();store.begin(&source.id,0,nonce.clone(),"haiku".into(),"Own instruction".into()).unwrap();
            let source=store.finish(&source.id,&nonce,Ok(crate::chat_api::Reply{text:"Own answer".into(),completed:true,note:None}),true).unwrap();
            let prefs=Preferences{quota_rules:vec![crate::settings::QuotaRule{provider:"Claude".into(),account:targets[0].account.clone(),thresholds:crate::settings::default_thresholds(),five_hour_trigger:Some(90),weekly_trigger:Some(80),fallback:vec![crate::settings::Destination{profile_id:"q".into(),model:"sonnet".into()},crate::settings::Destination{profile_id:"r".into(),model:"haiku".into()}]}],..Default::default()};
            Self{root,store,profiles,targets,prefs,source}
        }
        fn evaluate(&self,rows:&[Row],now:u64)->Vec<Status>{evaluate(&self.store,&self.profiles,&self.targets,rows,&self.prefs,now).unwrap()}
    }
    impl Drop for Fixture{fn drop(&mut self){let _=std::fs::remove_dir_all(&self.root);}}
    fn row(account:&str,minutes:u64,percent:f64)->Row{Row{provider:"Claude".into(),account:Some(account.into()),bucket:Some("subscription".into()),period:None,window:Some(crate::quotas::Window{used_percent:percent,window_duration_mins:minutes,resets_at:1000}),observed_at:Some(1000),state:"fresh".into(),message:String::new()}}
    #[test]
    fn chat_routing_prepares_at_either_window_without_send_or_replacing_review(){
        for (minutes,threshold) in [(300,90.0),(10080,80.0)]{
            let f=Fixture::new();assert!(f.evaluate(&[row(&f.source.target.account,minutes,threshold-1.0)],1000).is_empty());assert!(f.store.transfer_reviews().unwrap().is_empty());
            assert!(f.evaluate(&[row(&f.source.target.account,minutes,threshold)],1000).is_empty());let review=f.store.transfer_review(&f.source.id).unwrap();
            assert!(review.automatic);assert!(review.destination==f.targets[1]);assert_eq!(review.model,"sonnet");assert_eq!(review.source_revision,f.source.revision);
            assert_eq!(f.store.list().unwrap().len(),1);assert_eq!(f.store.get(&f.source.id).unwrap().used_nonces,f.source.used_nonces);
            f.evaluate(&[row(&f.source.target.account,minutes,threshold)],1001);assert_eq!(f.store.transfer_review(&f.source.id).unwrap().nonce,review.nonce);
            f.store.cancel_transfer(&f.source.id,&review.nonce).unwrap();f.evaluate(&[row(&f.source.target.account,minutes,threshold)],1002);assert!(f.store.transfer_reviews().unwrap().is_empty());
        }
    }
    #[test]
    fn chat_routing_waits_for_working_and_unknown_then_reviews_finished_turn(){
        let f=Fixture::new();let rows=[row(&f.source.target.account,300,90.0)];let nonce=uuid::Uuid::new_v4().to_string();
        f.store.begin(&f.source.id,f.source.revision,nonce.clone(),"haiku".into(),"Pending own turn".into()).unwrap();
        let statuses=f.evaluate(&rows,1000);assert_eq!(statuses.len(),1);assert_eq!(statuses[0].task_id,format!("chat:{}",f.source.id));assert_eq!(statuses[0].state,"waitingTurn");assert!(f.store.transfer_reviews().unwrap().is_empty());
        f.store.recover_interrupted().unwrap();assert_eq!(f.evaluate(&rows,1000)[0].state,"waitingTurn");assert!(f.store.transfer_reviews().unwrap().is_empty());
        let g=Fixture::new();let nonce=uuid::Uuid::new_v4().to_string();g.store.begin(&g.source.id,g.source.revision,nonce.clone(),"haiku".into(),"Next own turn".into()).unwrap();
        let finished=g.store.finish(&g.source.id,&nonce,Ok(crate::chat_api::Reply{text:"Finished own turn".into(),completed:true,note:None}),true).unwrap();
        g.evaluate(&[row(&g.source.target.account,300,90.0)],1000);assert_eq!(g.store.transfer_review(&g.source.id).unwrap().source_revision,finished.revision);
    }
    #[test]
    fn chat_routing_rejects_stale_samples_and_unconfirmed_source_identity(){
        let mut f=Fixture::new();
        for mode in 0..5{let mut sample=row(&f.source.target.account,300,95.0);let now=if mode==4{121001}else{1000};
            match mode{0=>sample.account=None,1=>sample.state="stale".into(),2=>sample.observed_at=Some(1001),3=>sample.window.as_mut().unwrap().window_duration_mins=30,_=>{}}
            assert!(f.evaluate(&[sample],now).is_empty());assert!(f.store.transfer_reviews().unwrap().is_empty());
        }
        f.targets[0].account="changed@example.invalid".into();let statuses=f.evaluate(&[row(&f.source.target.account,300,95.0)],1000);
        assert_eq!(statuses.len(),1);assert_eq!(statuses[0].state,"unavailable");assert!(f.store.transfer_reviews().unwrap().is_empty());assert_eq!(f.store.list().unwrap().len(),1);
    }
    #[test]
    fn chat_routing_respects_order_skips_exhausted_and_reports_no_alternative(){
        let f=Fixture::new();f.evaluate(&[row(&f.source.target.account,300,95.0),row(&f.targets[1].account,10080,100.0)],1000);
        let review=f.store.transfer_review(&f.source.id).unwrap();assert!(review.destination==f.targets[2]);assert_eq!(review.model,"haiku");
        let mut g=Fixture::new();g.targets.truncate(1);let statuses=g.evaluate(&[row(&g.source.target.account,300,95.0)],1000);
        assert_eq!(statuses.len(),1);assert_eq!(statuses[0].state,"exhausted");assert_eq!(statuses[0].task_id,format!("chat:{}",g.source.id));assert!(statuses[0].message.contains("Nenhuma alternativa"));assert!(g.store.transfer_reviews().unwrap().is_empty());
    }
    #[test]
    fn chat_routing_reviews_other_ai_without_send_and_requires_billing_consent(){
        for provider in ["OpenAI","Anthropic","Gemini"] {
            let mut f=Fixture::new();
            let target=Target{kind:"api".into(),profile_id:uuid::Uuid::new_v4().to_string(),provider:provider.into(),
                account:"Own API fixture".into(),billing:"api".into(),credential_revision:Some(uuid::Uuid::new_v4().to_string())};
            f.prefs.quota_rules[0].fallback=vec![crate::settings::Destination{profile_id:target.profile_id.clone(),model:"own-model".into()}];
            f.targets.push(target.clone());f.evaluate(&[row(&f.source.target.account,300,95.0)],1000);
            let review=f.store.transfer_review(&f.source.id).unwrap();assert!(review.automatic);assert!(review.destination==target);
            assert_eq!(f.store.list().unwrap().len(),1);assert_eq!(f.store.get(&f.source.id).unwrap().messages.len(),2);
            assert!(f.store.approve_transfer(&f.source.id,&review.nonce,review.summary.clone(),true,false,&f.source.target,&target).is_err());
            assert_eq!(f.store.list().unwrap().len(),1);
            let mut changed=target.clone();changed.credential_revision=Some(uuid::Uuid::new_v4().to_string());
            assert!(f.store.approve_transfer(&f.source.id,&review.nonce,review.summary.clone(),true,true,&f.source.target,&changed).is_err());
            assert_eq!(f.store.list().unwrap().len(),1);
        }
    }
    #[test]
    fn chat_routing_does_not_choose_an_unconfigured_api(){
        let mut f=Fixture::new();f.prefs.quota_rules[0].fallback=vec![crate::settings::Destination{profile_id:uuid::Uuid::new_v4().to_string(),model:"own-model".into()}];
        assert_eq!(f.evaluate(&[row(&f.source.target.account,300,95.0)],1000)[0].state,"exhausted");
        assert!(f.store.transfer_reviews().unwrap().is_empty());
    }
}
