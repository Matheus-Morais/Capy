use crate::{quota_policy, quotas::Row, settings::{Destination,QuotaRule}};
use serde::Serialize;

pub struct Candidate { pub profile_id:String, pub account:Option<String>, pub provider:String, pub ready:bool }
#[derive(Clone,Debug,PartialEq,Serialize)]
#[serde(tag="state",rename_all="camelCase")]
pub enum Decision {
    Idle,
    WaitingTurn,
    Review {destination:Destination},
    Exhausted,
}
#[derive(Clone,Debug,PartialEq,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Status {pub task_id:String,pub state:String,pub message:String}
pub fn decide(rule:&QuotaRule,source_profile:&str,source_model:&str,rows:&[Row],candidates:&[Candidate],turn_ended:bool,now:u64)->Decision{
    let triggered=rows.iter().filter(|r|r.provider==rule.provider&&r.account.as_deref()==Some(&rule.account)&&quota_policy::fresh(r,now)).any(|r|{
        let window=r.window.as_ref().unwrap();
        let threshold=match window.window_duration_mins {300=>rule.five_hour_trigger,10080=>rule.weekly_trigger,_=>None};
        threshold.is_some_and(|p|window.used_percent>=f64::from(p))
    });
    if !triggered{return Decision::Idle;}
    if !turn_ended{return Decision::WaitingTurn;}
    for destination in &rule.fallback {
        if destination.profile_id==source_profile&&destination.model==source_model{continue;}
        let Some(candidate)=candidates.iter().find(|c|c.profile_id==destination.profile_id&&c.ready)else{continue;};
        let exhausted=rows.iter().filter(|r|r.provider==candidate.provider&&r.account.as_deref()==candidate.account.as_deref()&&quota_policy::fresh(r,now)).any(|r|r.window.as_ref().unwrap().used_percent>=100.0);
        if !exhausted{return Decision::Review{destination:destination.clone()};}
    }
    Decision::Exhausted
}
#[cfg(test)]
mod tests{
    use super::*;use crate::{settings,quotas::Window};
    fn rule()->QuotaRule{QuotaRule{provider:"Claude".into(),account:"a".into(),thresholds:settings::default_thresholds(),five_hour_trigger:Some(90),weekly_trigger:Some(80),fallback:vec![Destination{profile_id:"p".into(),model:"sonnet".into()},Destination{profile_id:"q".into(),model:"opus".into()},Destination{profile_id:"r".into(),model:"sonnet".into()}]}}
    fn row(account:&str,minutes:u64,percent:f64)->Row{Row{provider:"Claude".into(),account:Some(account.into()),bucket:None,period:None,window:Some(Window{used_percent:percent,window_duration_mins:minutes,resets_at:1000}),observed_at:Some(1000),state:"fresh".into(),message:String::new()}}
    fn candidates()->Vec<Candidate>{vec![Candidate{profile_id:"p".into(),account:Some("a".into()),provider:"Claude".into(),ready:true},Candidate{profile_id:"q".into(),account:Some("b".into()),provider:"Claude".into(),ready:true},Candidate{profile_id:"r".into(),account:Some("c".into()),provider:"Claude".into(),ready:true}]}
    #[test]
    fn routing_respects_independent_thresholds_and_turn_boundary(){
        let rule=rule();let candidates=candidates();
        assert_eq!(decide(&rule,"p","sonnet",&[row("a",300,89.0),row("a",10080,79.0)],&candidates,true,1000),Decision::Idle);
        for rows in [vec![row("a",300,90.0)],vec![row("a",10080,80.0)]]{
            assert_eq!(decide(&rule,"p","sonnet",&rows,&candidates,false,1000),Decision::WaitingTurn);
            assert_eq!(decide(&rule,"p","sonnet",&rows,&candidates,true,1000),Decision::Review{destination:rule.fallback[1].clone()});
        }
    }
    #[test]
    fn routing_skips_unready_exhausted_and_unchanged_destinations(){
        let rule=rule();let mut candidates=candidates();
        let rows=vec![row("a",300,100.0),row("b",10080,100.0)];
        assert_eq!(decide(&rule,"p","sonnet",&rows,&candidates,true,1000),Decision::Review{destination:rule.fallback[2].clone()});
        candidates[2].ready=false;
        assert_eq!(decide(&rule,"p","sonnet",&rows,&candidates,true,1000),Decision::Exhausted);
        let mut changed=rule.clone();changed.fallback=vec![Destination{profile_id:"p".into(),model:"opus".into()}];
        assert_eq!(decide(&changed,"p","sonnet",&[row("a",300,90.0)],&candidates,true,1000),Decision::Review{destination:changed.fallback[0].clone()});
    }
    #[test]
    fn routing_never_uses_stale_missing_identity_or_unconfigured_trigger(){
        let rule=rule();let candidates=candidates();
        for mode in 0..4{
            let mut row=row("a",300,95.0);
            match mode{0=>row.state="stale".into(),1=>row.account=None,2=>row.observed_at=Some(1001),_=>row.window.as_mut().unwrap().window_duration_mins=30};
            assert_eq!(decide(&rule,"p","sonnet",&[row],&candidates,true,1000),Decision::Idle);
        }
        assert_eq!(decide(&rule,"p","sonnet",&[row("a",300,95.0)],&candidates,true,121001),Decision::Idle);
        let mut manual=rule;manual.five_hour_trigger=None;
        assert_eq!(decide(&manual,"p","sonnet",&[row("a",300,95.0)],&candidates,true,1000),Decision::Idle);
    }
}
