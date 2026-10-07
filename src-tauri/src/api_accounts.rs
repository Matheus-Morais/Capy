use crate::{credential_vault::{self,Credential},settings};
use serde::{Deserialize,Serialize};
use std::{path::PathBuf,sync::Mutex};

#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Account {pub id:String,pub label:String,pub provider:String,pub revision:String}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct View {pub account:Account,pub configured:bool,pub billing:&'static str}
pub struct Store {path:PathBuf,accounts:Mutex<Vec<Account>>,load_error:bool}
impl Store {
    pub fn load(path:PathBuf)->Self{
        if !path.exists(){return Self{path,accounts:Mutex::new(Vec::new()),load_error:false};}
        match settings::read_json::<Vec<Account>>(&path){
            Ok(accounts) if valid_accounts(&accounts)=>Self{path,accounts:Mutex::new(accounts),load_error:false},
            _=>Self{path,accounts:Mutex::new(Vec::new()),load_error:true},
        }
    }
    pub fn list(&self)->Result<Vec<View>,String>{
        if self.load_error{return Err("O cadastro de APIs não pôde ser lido; o arquivo foi preservado.".into());}
        Ok(self.accounts.lock().map_err(|_|"Contas indisponíveis.")?.iter().map(|account|View{account:account.clone(),configured:self.key(account).is_ok(),billing:"api"}).collect())
    }
    pub fn get(&self,id:&str)->Result<Account,String>{
        if self.load_error{return Err("O cadastro de APIs não pôde ser lido.".into());}
        self.accounts.lock().map_err(|_|"Contas indisponíveis.")?.iter().find(|account|account.id==id).cloned().ok_or("Conta API não encontrada.".into())
    }
    pub fn key(&self,account:&Account)->Result<String,String>{
        let credential=credential_vault::read(&account.id)?;
        if credential.revision!=account.revision||!valid_key(&credential.key){return Err("A chave mudou ou é inválida. Confirme novamente a conta antes de enviar.".into());}
        Ok(credential.key)
    }
    pub fn add(&self,label:String,provider:String,key:String)->Result<View,String>{
        if self.load_error{return Err("Recupere o cadastro de APIs antes de adicionar uma conta.".into());}
        if !settings::valid_text(&label,80)||!providers().contains(&provider.as_str())||!valid_key(&key){return Err("Nome, provedor ou chave inválidos.".into());}
        let mut accounts=self.accounts.lock().map_err(|_|"Contas indisponíveis.")?;
        if accounts.len()>=64{return Err("Limite de 64 contas API.".into());}
        let account=Account{id:uuid::Uuid::new_v4().to_string(),label,provider,revision:uuid::Uuid::new_v4().to_string()};
        credential_vault::write(&account.id,&Credential{revision:account.revision.clone(),key})?;
        let mut updated=accounts.clone();updated.push(account.clone());
        if let Err(error)=settings::write_json(&self.path,&updated){let _=credential_vault::delete(&account.id);return Err(error);}
        *accounts=updated;Ok(View{account,configured:true,billing:"api"})
    }
}
pub fn providers()->[&'static str;3]{["OpenAI","Anthropic","Gemini"]}
fn valid(account:&Account)->bool{crate::discovery::uuid(&account.id)&&crate::discovery::uuid(&account.revision)&&settings::valid_text(&account.label,80)&&providers().contains(&account.provider.as_str())}
fn valid_accounts(accounts:&[Account])->bool{
    let mut ids=std::collections::HashSet::new();
    accounts.len()<=64&&accounts.iter().all(|account|valid(account)&&ids.insert(&account.id))
}
fn valid_key(key:&str)->bool{(8..=2048).contains(&key.len())&&key.bytes().all(|b|b.is_ascii_graphic())}

#[cfg(all(test,windows))]
mod tests {
    use super::*;
    #[test]
    fn chat_api_account_json_never_contains_key_and_preserves_multiple_accounts(){
        let root=std::env::temp_dir().join(format!("capy-api-{}",uuid::Uuid::new_v4()));
        let store=Store::load(root.join("api-accounts.json"));
        let first=store.add("Personal".into(),"OpenAI".into(),"capy-dummy-key-1-not-real".into()).unwrap();
        let second=store.add("Work".into(),"OpenAI".into(),"capy-dummy-key-2-not-real".into()).unwrap();
        assert_ne!(first.account.id,second.account.id);assert_eq!(Store::load(store.path.clone()).list().unwrap().len(),2);
        assert!(!std::fs::read_to_string(&store.path).unwrap().contains("capy-dummy-key"));
        let mut changed=first.account.clone();changed.revision=uuid::Uuid::new_v4().to_string();assert!(store.key(&changed).is_err());
        credential_vault::delete(&first.account.id).unwrap();credential_vault::delete(&second.account.id).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn chat_api_accounts_preserve_invalid_metadata_instead_of_resetting(){
        let root=std::env::temp_dir().join(format!("capy-api-invalid-{}",uuid::Uuid::new_v4()));
        let path=root.join("api-accounts.json");
        std::fs::create_dir_all(&root).unwrap();std::fs::write(&path,b"{broken").unwrap();
        let store=Store::load(path.clone());
        assert!(store.list().is_err());assert!(store.add("Name".into(),"OpenAI".into(),"dummy-key-for-test".into()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(),b"{broken");
        let account=Account{id:uuid::Uuid::new_v4().to_string(),label:"Name".into(),provider:"OpenAI".into(),revision:uuid::Uuid::new_v4().to_string()};
        settings::write_json(&path,&vec![account.clone(),account]).unwrap();
        assert!(Store::load(path).list().is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
