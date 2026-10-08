use std::{io::Read,path::PathBuf,process::Command,sync::mpsc,time::{Duration,Instant}};
use time::{format_description::well_known::Rfc3339,OffsetDateTime};

const LIMIT:u64=65_536;
fn executable() -> Option<PathBuf> {
    let installed=std::env::var_os("LOCALAPPDATA").map(|dir|PathBuf::from(dir).join("agy/bin/agy.exe"));
    installed.filter(|p|p.is_file()).or_else(||std::env::var_os("PATH").and_then(|paths|
        std::env::split_paths(&paths).map(|dir|dir.join("agy.exe")).find(|p|p.is_absolute()&&p.is_file())))
        .and_then(|path|path.canonicalize().ok())
}
pub fn collect(export_ready:impl Fn()->bool) -> Result<(),&'static str> {
    let exe=executable().ok_or("CLI agy não encontrado. Instale o CLI oficial para coleta automática.")?;
    let mut command=Command::new(exe);
    command.args(["--print","/usage","--print-timeout","15s"]).current_dir(std::env::temp_dir());
    let output=run(command,Duration::from_secs(30),export_ready)?;
    if !valid_report(&output) {return Err("O agy retornou um relatório de cotas incompatível; amostra não renovada.");}
    Ok(())
}
fn valid_report(bytes:&[u8]) -> bool {
    let Ok(text)=std::str::from_utf8(bytes)else{return false;};
    let mut windows=std::collections::HashSet::new();
    for line in text.lines().filter(|line|!line.trim().is_empty()) {
        let fields=line.split('\t').collect::<Vec<_>>();
        if fields.len()!=4 {return false;}
        let group=match fields[0] {"Gemini Models"=>"gemini","Claude and GPT models"=>"other",_=>return false};
        let period=match fields[1] {"Weekly Limit Remaining"=>"week","Five Hour Limit Remaining"=>"five",_=>return false};
        if !windows.insert((group,period)) {return false;}
        if fields[2]=="disabled" {continue;}
        if !fields[2].strip_suffix('%').and_then(|value|value.parse::<f64>().ok()).is_some_and(|v|v.is_finite()&&(0.0..=100.0).contains(&v))
            || OffsetDateTime::parse(fields[3],&Rfc3339).is_err() {return false;}
    }
    windows.len()==4
}
fn run(mut command:Command,timeout:Duration,export_ready:impl Fn()->bool) -> Result<Vec<u8>,&'static str> {
    let mut child=crate::chat_process::spawn(&mut command).map_err(|_|"Não foi possível iniciar a consulta agy em segundo plano.")?;
    drop(child.stdin.take());
    let stdout=child.stdout.take().ok_or("Saída da consulta agy indisponível.")?;
    let (sender,receiver)=mpsc::channel();
    std::thread::spawn(move||{
        let mut bytes=Vec::new();let result=stdout.take(LIMIT+1).read_to_end(&mut bytes).map(|_|bytes);
        let _=sender.send(result);
    });
    let started=Instant::now();let mut output=None;
    let status=loop {
        if let Ok(result)=receiver.try_recv() {
            let bytes=result.map_err(|_|"Falha ao ler a consulta agy.")?;
            if bytes.len()>LIMIT as usize {return Err("Consulta agy excedeu 64 KiB; processo próprio encerrado.");}
            output=Some(bytes);
        }
        match child.try_wait() {
            Ok(Some(status))=>break status,
            Ok(None) if started.elapsed()<timeout=>std::thread::sleep(Duration::from_millis(25)),
            _=>return Err("Consulta agy excedeu o prazo; processo próprio encerrado, saldo não renovado."),
        }
    };
    if !status.success() {return Err("Consulta automática agy falhou. Confira o login no CLI oficial.");}
    while !export_ready() {
        if started.elapsed()>=timeout {return Err("O agy respondeu, mas não exportou uma nova amostra identificada no prazo.");}
        std::thread::sleep(Duration::from_millis(50));
    }
    drop(child);
    let bytes=match output {Some(bytes)=>bytes,None=>receiver.recv_timeout(Duration::from_secs(2))
        .map_err(|_|"Saída da consulta agy não terminou no prazo.")?.map_err(|_|"Falha ao ler a consulta agy.")?};
    if bytes.len()>LIMIT as usize {return Err("Consulta agy excedeu 64 KiB; saldo não renovado.");}
    Ok(bytes)
}

#[cfg(test)] mod tests {
    use super::*;
    const REPORT:&str="Gemini Models\tWeekly Limit Remaining\t47%\t2026-10-09T19:20:26Z\nGemini Models\tFive Hour Limit Remaining\t39%\t2026-10-08T17:22:01Z\nClaude and GPT models\tWeekly Limit Remaining\t0%\t2026-10-10T16:52:06Z\nClaude and GPT models\tFive Hour Limit Remaining\tdisabled\t\n";
    #[test] fn agy_usage_rejects_model_output_and_malformed_reports() {
        assert!(valid_report(REPORT.as_bytes()));
        for bad in ["","A resposta do modelo foi 47%",&REPORT.replace("47%","101%"),&REPORT.replace("47%","NaN%"),
            &REPORT.replace("2026-10-09T19:20:26Z","invalid"),&REPORT.replace("Weekly Limit Remaining","Five Hour Limit Remaining")] {
            assert!(!valid_report(bad.as_bytes()));
        }
    }
    #[cfg(windows)]
    #[test] fn agy_usage_bounds_process_output_and_timeout() {
        let shell=PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let command=|script:&str|{let mut c=Command::new(&shell);c.args(["-NoProfile","-NonInteractive","-Command",script]);c};
        assert_eq!(run(command("[Console]::Write('own fixture')"),Duration::from_secs(10),||true).unwrap(),b"own fixture");
        assert!(run(command("[Console]::Write(('x' * 65537))"),Duration::from_secs(10),||true).unwrap_err().contains("64 KiB"));
        let start=Instant::now();assert!(run(command("Start-Sleep -Seconds 30"),Duration::from_millis(250),||true).unwrap_err().contains("prazo"));
        assert!(start.elapsed()<Duration::from_secs(3));
        assert!(run(command("exit 1"),Duration::from_secs(10),||true).is_err());
        assert!(run(command("exit 0"),Duration::from_millis(250),||false).is_err());
    }
}
