#[cfg(windows)]
mod windows {
    use std::{collections::BTreeMap,ffi::{OsStr,OsString},fs::File,io,mem::{size_of,zeroed},
        os::windows::{ffi::OsStrExt,io::{AsRawHandle,FromRawHandle}},path::Path,process::{Command,ExitStatus},ptr};
    use windows_sys::Win32::{Foundation::*,Security::SECURITY_ATTRIBUTES,System::{
        JobObjects::*,Pipes::CreatePipe,Threading::*}};

    struct Handle(HANDLE);
    impl Handle {
        fn checked(raw:HANDLE)->io::Result<Self>{
            if raw.is_null()||raw==INVALID_HANDLE_VALUE {Err(io::Error::last_os_error())}else{Ok(Self(raw))}
        }
        fn into_file(self)->File{let raw=self.0;std::mem::forget(self);unsafe{File::from_raw_handle(raw)}}
    }
    impl Drop for Handle {fn drop(&mut self){unsafe{CloseHandle(self.0);}}}

    struct Attributes {storage:Vec<usize>,initialized:bool}
    impl Attributes {
        fn new()->io::Result<Self>{
            let mut bytes=0;
            unsafe{InitializeProcThreadAttributeList(ptr::null_mut(),2,0,&mut bytes);}
            if bytes==0{return Err(io::Error::last_os_error());}
            let mut result=Self{storage:vec![0;bytes.div_ceil(size_of::<usize>())],initialized:false};
            if unsafe{InitializeProcThreadAttributeList(result.raw(),2,0,&mut bytes)}==0 {
                return Err(io::Error::last_os_error());
            }
            result.initialized=true;Ok(result)
        }
        fn raw(&mut self)->LPPROC_THREAD_ATTRIBUTE_LIST {self.storage.as_mut_ptr().cast()}
        fn add(&mut self,key:usize,handles:&[HANDLE])->io::Result<()>{
            if unsafe{UpdateProcThreadAttribute(self.raw(),0,key,handles.as_ptr().cast(),std::mem::size_of_val(handles),ptr::null_mut(),ptr::null())}==0 {
                Err(io::Error::last_os_error())
            }else{Ok(())}
        }
    }
    impl Drop for Attributes {fn drop(&mut self){if self.initialized{unsafe{DeleteProcThreadAttributeList(self.raw());}}}}

    fn wide(value:&OsStr)->io::Result<Vec<u16>>{
        let mut result:Vec<u16>=value.encode_wide().collect();
        if result.contains(&0){return Err(io::Error::new(io::ErrorKind::InvalidInput,"NUL in process argument"));}
        result.push(0);Ok(result)
    }
    fn quoted(value:&OsStr)->io::Result<Vec<u16>>{
        let source=wide(value)?;let mut result=vec![34];let mut slashes=0;
        for &unit in &source[..source.len()-1]{
            if unit==92 {slashes+=1;continue;}
            result.extend(std::iter::repeat_n(92,if unit==34 {slashes*2+1}else{slashes}));
            slashes=0;result.push(unit);
        }
        result.extend(std::iter::repeat_n(92,slashes*2));result.push(34);Ok(result)
    }
    fn environment(command:&Command)->io::Result<Vec<u16>>{
        let mut entries:BTreeMap<String,(OsString,OsString)>=std::env::vars_os()
            .map(|(key,value)|(key.to_string_lossy().to_uppercase(),(key,value))).collect();
        for (key,value) in command.get_envs(){
            let normalized=key.to_string_lossy().to_uppercase();
            if let Some(value)=value {entries.insert(normalized,(key.into(),value.into()));}else{entries.remove(&normalized);}
        }
        let mut result=Vec::new();
        for (_, (key,value)) in entries{
            let key=wide(&key)?;let value=wide(&value)?;
            result.extend_from_slice(&key[..key.len()-1]);result.push(61);result.extend(value);
        }
        if result.is_empty(){result.push(0);}result.push(0);Ok(result)
    }
    fn pipe(parent_reads:bool)->io::Result<(Handle,Handle)>{
        let mut read=ptr::null_mut();let mut write=ptr::null_mut();
        let attributes=SECURITY_ATTRIBUTES{nLength:size_of::<SECURITY_ATTRIBUTES>() as u32,lpSecurityDescriptor:ptr::null_mut(),bInheritHandle:1};
        if unsafe{CreatePipe(&mut read,&mut write,&attributes,0)}==0{return Err(io::Error::last_os_error());}
        let read=Handle(read);let write=Handle(write);
        let (parent,child)=if parent_reads{(read,write)}else{(write,read)};
        if unsafe{SetHandleInformation(parent.0,HANDLE_FLAG_INHERIT,0)}==0{return Err(io::Error::last_os_error());}
        Ok((parent,child))
    }

    pub struct OwnedChild {
        process:Handle,
        job:Option<Handle>,
        pub stdin:Option<File>,
        pub stdout:Option<File>,
    }
    impl OwnedChild {
        pub fn try_wait(&mut self)->io::Result<Option<ExitStatus>>{
            use std::os::windows::process::ExitStatusExt;
            match unsafe{WaitForSingleObject(self.process.0,0)} {
                WAIT_TIMEOUT=>Ok(None),
                WAIT_OBJECT_0=>{
                    let mut code=0;
                    if unsafe{GetExitCodeProcess(self.process.0,&mut code)}==0{return Err(io::Error::last_os_error());}
                    Ok(Some(ExitStatus::from_raw(code)))
                }
                _=>Err(io::Error::last_os_error()),
            }
        }
    }
    impl Drop for OwnedChild {
        fn drop(&mut self){
            // The private job also owns descendants; no process lookup or PID reuse.
            drop(self.job.take());unsafe{WaitForSingleObject(self.process.0,5000);}
        }
    }

    pub fn spawn(command:&mut Command)->io::Result<OwnedChild>{
        let executable=Path::new(command.get_program());
        if !executable.is_absolute()||!executable.extension().is_some_and(|s|s.eq_ignore_ascii_case("exe")){
            return Err(io::Error::new(io::ErrorKind::InvalidInput,"Chat requires an absolute executable"));
        }
        let program=wide(command.get_program())?;
        let mut line=quoted(command.get_program())?;
        for arg in command.get_args(){line.push(32);line.extend(quoted(arg)?);}
        line.push(0);
        if line.len()>32767{return Err(io::Error::new(io::ErrorKind::InvalidInput,"Process command line exceeds Windows limit"));}
        let cwd=wide(command.get_current_dir().unwrap_or(&std::env::current_dir()? ).as_os_str())?;
        let env=environment(command)?;
        let job=Handle::checked(unsafe{CreateJobObjectW(ptr::null(),ptr::null())})?;
        let mut limits:JOBOBJECT_EXTENDED_LIMIT_INFORMATION=unsafe{zeroed()};
        limits.BasicLimitInformation.LimitFlags=JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe{SetInformationJobObject(job.0,JobObjectExtendedLimitInformation,(&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32)}==0{return Err(io::Error::last_os_error());}
        let (input,child_input)=pipe(false)?;let (output,child_output)=pipe(true)?;
        let nul=std::fs::OpenOptions::new().write(true).open("NUL")?;let mut error=ptr::null_mut();
        if unsafe{DuplicateHandle(GetCurrentProcess(),nul.as_raw_handle(),GetCurrentProcess(),&mut error,0,1,DUPLICATE_SAME_ACCESS)}==0{return Err(io::Error::last_os_error());}
        let error=Handle(error);
        let jobs=[job.0];let inherited=[child_input.0,child_output.0,error.0];
        let mut attributes=Attributes::new()?;
        attributes.add(PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,&jobs)?;
        attributes.add(PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,&inherited)?;
        let mut startup:STARTUPINFOEXW=unsafe{zeroed()};
        startup.StartupInfo.cb=size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags=STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput=child_input.0;startup.StartupInfo.hStdOutput=child_output.0;startup.StartupInfo.hStdError=error.0;
        startup.lpAttributeList=attributes.raw();
        let mut info:PROCESS_INFORMATION=unsafe{zeroed()};
        let created=unsafe{CreateProcessW(program.as_ptr(),line.as_mut_ptr(),ptr::null(),ptr::null(),1,
            CREATE_NO_WINDOW|CREATE_UNICODE_ENVIRONMENT|EXTENDED_STARTUPINFO_PRESENT,env.as_ptr().cast(),cwd.as_ptr(),&startup.StartupInfo,&mut info)};
        if created==0{return Err(io::Error::last_os_error());}
        let process=Handle(info.hProcess);let _thread=Handle(info.hThread);
        Ok(OwnedChild{process,job:Some(job),stdin:Some(input.into_file()),stdout:Some(output.into_file())})
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{thread,time::{Duration,Instant}};
        fn shell()->std::path::PathBuf{
            std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/WindowsPowerShell/v1.0/powershell.exe")
        }
        struct FixtureChild(std::process::Child);
        impl Drop for FixtureChild{fn drop(&mut self){let _=self.0.kill();let _=self.0.wait();}}

        #[test]
        fn chat_process_job_owner_fixture(){
            let Some(root)=std::env::var_os("CAPY_CHAT_JOB_PROOF_ROOT")else{return;};
            let root=std::path::PathBuf::from(root);
            assert!(root.file_name().unwrap().to_string_lossy().starts_with("capy-job-proof-"));
            let mut command=Command::new(shell());
            command.args(["-NoProfile","-NonInteractive","-Command",
                "$child=Start-Process -FilePath (Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe') -ArgumentList @('-NoProfile','-NonInteractive','-Command','Start-Sleep -Seconds 60') -WindowStyle Hidden -PassThru; @{child=$PID;grandchild=$child.Id} | ConvertTo-Json -Compress | Set-Content -LiteralPath $env:CAPY_CHAT_JOB_PROOF_RECORD -Encoding ASCII; Start-Sleep -Seconds 60"])
                .env("CAPY_CHAT_JOB_PROOF_RECORD",root.join("pids.json"));
            let _owned=spawn(&mut command).unwrap();
            thread::sleep(Duration::from_secs(60));
        }

        #[test]
        fn chat_process_job_kills_only_owned_tree_when_owner_is_terminated(){
            use std::os::windows::process::CommandExt;
            let root=std::env::temp_dir().join(format!("capy-job-proof-{}",uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            let sentinel=Command::new(shell()).creation_flags(CREATE_NO_WINDOW)
                .args(["-NoProfile","-NonInteractive","-Command","Start-Sleep -Seconds 60"])
                .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
            let mut sentinel=FixtureChild(sentinel);
            let helper=Command::new(std::env::current_exe().unwrap()).creation_flags(CREATE_NO_WINDOW)
                .args(["--exact","chat_process::windows::tests::chat_process_job_owner_fixture","--nocapture"])
                .env("CAPY_CHAT_JOB_PROOF_ROOT",&root)
                .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap();
            let mut helper=FixtureChild(helper);
            let at=Instant::now();let pids=loop{
                if let Ok(bytes)=std::fs::read(root.join("pids.json")){
                    if let Ok(value)=serde_json::from_slice::<serde_json::Value>(&bytes){break [value["child"].as_u64().unwrap() as u32,value["grandchild"].as_u64().unwrap() as u32];}
                }
                assert!(at.elapsed()<Duration::from_secs(20),"Own fixture did not publish processes");
                assert!(helper.0.try_wait().unwrap().is_none(),"Fixture owner exited unexpectedly");
                thread::sleep(Duration::from_millis(25));
            };
            let births=pids.map(|pid|crate::discovery::process_birth(pid).expect("Child must be running before forced owner termination"));
            helper.0.kill().unwrap();helper.0.wait().unwrap();
            let at=Instant::now();
            while pids.iter().zip(births).any(|(&pid,birth)|crate::discovery::process_birth(pid)==Some(birth)) {
                assert!(at.elapsed()<Duration::from_secs(5),"Owned process survived parent termination");
                thread::sleep(Duration::from_millis(25));
            }
            assert!(sentinel.0.try_wait().unwrap().is_none(),"Unrelated same-name process must survive");
            drop(sentinel);std::fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn chat_process_quotes_empty_quotes_and_trailing_slashes(){
            let quoted_text=|text:&str|String::from_utf16(&quoted(OsStr::new(text)).unwrap()).unwrap();
            assert_eq!(quoted_text(""),"\"\"");
            assert_eq!(quoted_text("a b"),"\"a b\"");
            assert_eq!(quoted_text("a\"b"),"\"a\\\"b\"");
            assert_eq!(quoted_text("C:\\with space\\"),"\"C:\\with space\\\\\"");
            assert!(quoted(OsStr::new("bad\0argument")).is_err());
            let mut command=Command::new(shell());
            command.env("CAPY_JOB_ENV_PROOF","literal é `$(no-command)`").env_remove("ANTHROPIC_API_KEY");
            let block=String::from_utf16(&environment(&command).unwrap()).unwrap();
            assert!(block.contains("CAPY_JOB_ENV_PROOF=literal é `$(no-command)`\0"));
            assert!(!block.split('\0').any(|entry|entry.to_uppercase().starts_with("ANTHROPIC_API_KEY=")));
            assert!(block.ends_with("\0\0"));
        }
    }
}
#[cfg(windows)]
pub use windows::spawn;

#[cfg(not(windows))]
pub fn spawn(_: &mut std::process::Command)->std::io::Result<std::process::Child>{
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"Chat process containment requires Windows"))
}
