use serde::{Deserialize,Serialize};

#[derive(Deserialize,Serialize)]
pub struct Credential {pub revision:String,pub key:String}

fn target(id:&str)->Result<String,String>{
    if !crate::discovery::uuid(id){return Err("Identidade de chave inválida.".into());}
    Ok(format!("Capy/chat/{id}"))
}

#[cfg(windows)]
mod platform {
    use super::*;
    use windows_sys::Win32::Security::Credentials::{CredReadW,CredWriteW,CredDeleteW,CredFree,CREDENTIALW,CRED_TYPE_GENERIC,CRED_PERSIST_LOCAL_MACHINE};
    fn wide(text:&str)->Vec<u16>{text.encode_utf16().chain(Some(0)).collect()}
    pub fn write(id:&str,credential:&Credential)->Result<(),String>{
        let mut name=wide(&target(id)?);let mut username=wide("Capy API");
        let mut bytes=serde_json::to_vec(credential).map_err(|_|"Chave inválida.")?;
        if bytes.len()>2560{return Err("Chave excede o limite do cofre do Windows.".into());}
        let mut record:CREDENTIALW=unsafe{std::mem::zeroed()};
        record.Type=CRED_TYPE_GENERIC;record.TargetName=name.as_mut_ptr();record.UserName=username.as_mut_ptr();
        record.CredentialBlobSize=bytes.len() as u32;record.CredentialBlob=bytes.as_mut_ptr();record.Persist=CRED_PERSIST_LOCAL_MACHINE;
        let ok=unsafe{CredWriteW(&record,0)}!=0;bytes.fill(0);
        if ok{Ok(())}else{Err("O Windows não permitiu salvar a chave no cofre de credenciais.".into())}
    }
    pub fn read(id:&str)->Result<Credential,String>{
        let name=wide(&target(id)?);let mut pointer: *mut CREDENTIALW=std::ptr::null_mut();
        if unsafe{CredReadW(name.as_ptr(),CRED_TYPE_GENERIC,0,&mut pointer)}==0||pointer.is_null(){return Err("Chave não encontrada no cofre do Windows.".into());}
        struct Owned(*mut CREDENTIALW);
        impl Drop for Owned{fn drop(&mut self){unsafe{CredFree(self.0.cast());}}}
        let owned=Owned(pointer);let record=unsafe{&*owned.0};
        if record.Type!=CRED_TYPE_GENERIC||record.CredentialBlob.is_null()||record.CredentialBlobSize>2560{return Err("Registro de chave incompatível.".into());}
        let bytes=unsafe{std::slice::from_raw_parts(record.CredentialBlob,record.CredentialBlobSize as usize)};
        serde_json::from_slice(bytes).map_err(|_|"Registro de chave incompatível.".into())
    }
    pub fn delete(id:&str)->Result<(),String>{
        let name=wide(&target(id)?);
        if unsafe{CredDeleteW(name.as_ptr(),CRED_TYPE_GENERIC,0)}!=0{Ok(())}else{Err("Não foi possível remover a chave própria da Capy.".into())}
    }
}
#[cfg(not(windows))]
mod platform {
    use super::*;
    pub fn write(_: &str,_:&Credential)->Result<(),String>{Err("O cofre de chaves requer Windows.".into())}
    pub fn read(_: &str)->Result<Credential,String>{Err("O cofre de chaves requer Windows.".into())}
    pub fn delete(_: &str)->Result<(),String>{Err("O cofre de chaves requer Windows.".into())}
}
pub use platform::{read,write,delete};

#[cfg(all(test,windows))]
mod tests {
    use super::*;
    #[test]
    fn chat_api_keys_roundtrip_only_in_own_windows_vault_target(){
        let id=uuid::Uuid::new_v4().to_string();let revision=uuid::Uuid::new_v4().to_string();
        write(&id,&Credential{revision:revision.clone(),key:"capy-test-dummy-key-no-service".into()}).unwrap();
        let restored=read(&id).unwrap();assert_eq!(restored.key,"capy-test-dummy-key-no-service");assert_eq!(restored.revision,revision);
        assert!(read("../other-account").is_err());assert!(delete("WindowsLive:(token)").is_err());
        delete(&id).unwrap();assert!(read(&id).is_err());
    }
}
