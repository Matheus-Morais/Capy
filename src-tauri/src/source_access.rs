use crate::{demo::Session, discovery::Report};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct SourceAction {
    pub label: String,
    pub available: bool,
    pub reason: Option<String>,
}

fn codex_url(session: &Session) -> Option<String> {
    let id = session.id.strip_prefix("codex:")?;
    (session.kind == "codex" && crate::discovery::uuid(id)).then(|| format!("codex://threads/{id}"))
}

pub fn enrich(report: &mut Report) {
    let available = handler_available();
    for session in &mut report.sessions {
        session.source_action = codex_url(session).map(|_| SourceAction {
            label: "Abrir no Codex".into(),
            available,
            reason: (!available).then(|| "O link do Codex não está registrado neste Windows. Abra a conversa no aplicativo de origem.".into()),
        });
    }
}

fn open_verified(
    expected: &Session,
    real: bool,
    current: &Report,
    handler: bool,
    dispatch: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    if !real {
        return Err("A demonstração não abre sessões reais.".into());
    }
    let url =
        codex_url(expected).ok_or("Acesso à origem ainda não disponível para esta sessão.")?;
    if !current
        .sessions
        .iter()
        .any(|s| s.id == expected.id && s.kind == expected.kind && s.origin == expected.origin)
    {
        return Err("Esta sessão encerrou ou mudou de projeto. Atualize a lista.".into());
    }
    if !handler {
        return Err("O link do Codex não está registrado neste Windows.".into());
    }
    dispatch(&url)
}

pub fn open(expected: &Session, real: bool, current: &Report) -> Result<(), String> {
    open_verified(expected, real, current, handler_available(), dispatch)
}

#[cfg(windows)]
pub fn handler_available() -> bool {
    use windows_sys::Win32::UI::Shell::{
        AssocQueryStringW, ASSOCF_IS_PROTOCOL, ASSOCSTR_APPID, ASSOCSTR_EXECUTABLE,
    };
    let scheme: Vec<u16> = "codex\0".encode_utf16().collect();
    registered_handler(|packaged| {
        let mut output = vec![0u16; 32_768];
        let mut len = output.len() as u32;
        unsafe {
            AssocQueryStringW(
                ASSOCF_IS_PROTOCOL,
                if packaged {
                    ASSOCSTR_APPID
                } else {
                    ASSOCSTR_EXECUTABLE
                },
                scheme.as_ptr(),
                std::ptr::null(),
                output.as_mut_ptr(),
                &mut len,
            ) == 0
                && len > 1
                && len <= output.len() as u32
                && output[0] != 0
        }
    })
}

fn registered_handler(query: impl Fn(bool) -> bool) -> bool {
    query(false) || query(true)
}

#[cfg(windows)]
fn dispatch(url: &str) -> Result<(), String> {
    use windows_sys::Win32::System::Com::{
        CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
    };
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    let operation: Vec<u16> = "open\0".encode_utf16().collect();
    let destination: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
    let com = unsafe {
        CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        )
    };
    if com < 0 {
        return Err("Não foi possível preparar a abertura no Windows. Abra a conversa no aplicativo de origem.".into());
    }
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            destination.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    unsafe {
        CoUninitialize();
    }
    dispatched(result)
}

fn dispatched(result: isize) -> Result<(), String> {
    if result > 32 {
        Ok(())
    } else {
        Err(format!("O Windows não conseguiu abrir a conversa no Codex (código {result}). Abra-a no aplicativo de origem."))
    }
}

#[cfg(not(windows))]
pub fn handler_available() -> bool {
    false
}
#[cfg(not(windows))]
fn dispatch(_: &str) -> Result<(), String> {
    Err("Abertura disponível somente no Windows.".into())
}

#[cfg(test)]
mod tests;
