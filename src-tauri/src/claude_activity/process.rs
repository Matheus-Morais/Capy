use crate::discovery::process_birth;
use std::collections::HashMap;
use windows_sys::Win32::{
    Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION},
    },
};

pub(super) fn claude_ancestor() -> Option<(u32, u64)> {
    ancestor_named(&["claude.exe"])
}

pub(crate) fn ancestor_named(names: &[&str]) -> Option<(u32, u64)> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut parents = HashMap::new();
        let mut ok = Process32FirstW(snapshot, &mut entry);
        while ok != 0 {
            parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
            ok = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
        let mut pid = std::process::id();
        let mut child_birth = process_birth(pid)?;
        for _ in 0..8 {
            let parent = *parents.get(&pid)?;
            let parent_birth = process_birth(parent)?;
            if parent_birth > child_birth || parent == pid {
                return None;
            }
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, parent);
            if handle.is_null() {
                return None;
            }
            let mut path = vec![0u16; 32_768];
            let mut len = path.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, path.as_mut_ptr(), &mut len) != 0;
            CloseHandle(handle);
            if !ok {
                return None;
            }
            let path = String::from_utf16(&path[..len as usize]).ok()?;
            if names.iter().any(|name| {
                path.rsplit(['\\', '/'])
                    .next()
                    .is_some_and(|actual| actual.eq_ignore_ascii_case(name))
            }) {
                return Some((parent, parent_birth));
            }
            pid = parent;
            child_birth = parent_birth;
        }
        None
    }
}
