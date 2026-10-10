"""Isolated Windows ConPTY QA host; no third-party packages.

Follows https://learn.microsoft.com/windows/console/creating-a-pseudoconsole-session.
Input is consumed from a JSON file containing an array of strings, appended by
the orchestrator. Strings are sent unchanged (use \r for Enter).
"""
import argparse
import ctypes as c
from ctypes import wintypes as w
import json
from pathlib import Path
import subprocess
import threading
import time


class COORD(c.Structure):
    _fields_ = [('X', c.c_short), ('Y', c.c_short)]


class STARTUPINFO(c.Structure):
    _fields_ = [('cb', w.DWORD), ('lpReserved', w.LPWSTR),
                ('lpDesktop', w.LPWSTR), ('lpTitle', w.LPWSTR),
                ('dwX', w.DWORD), ('dwY', w.DWORD), ('dwXSize', w.DWORD),
                ('dwYSize', w.DWORD), ('dwXCountChars', w.DWORD),
                ('dwYCountChars', w.DWORD), ('dwFillAttribute', w.DWORD),
                ('dwFlags', w.DWORD), ('wShowWindow', w.WORD),
                ('cbReserved2', w.WORD), ('lpReserved2', c.POINTER(c.c_byte)),
                ('hStdInput', w.HANDLE), ('hStdOutput', w.HANDLE),
                ('hStdError', w.HANDLE)]


class STARTUPINFOEX(c.Structure):
    _fields_ = [('StartupInfo', STARTUPINFO), ('lpAttributeList', c.c_void_p)]


class PROCESS_INFORMATION(c.Structure):
    _fields_ = [('hProcess', w.HANDLE), ('hThread', w.HANDLE),
                ('dwProcessId', w.DWORD), ('dwThreadId', w.DWORD)]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', required=True)
    parser.add_argument('--script', required=True)
    parser.add_argument('--timeout', type=int, default=180)
    args = parser.parse_args()
    root = Path(args.root).resolve()
    script = Path(args.script).resolve()
    if not root.is_dir() or not script.is_file():
        raise RuntimeError('Existing isolated root and launch script required')
    kernel = c.WinDLL('kernel32', use_last_error=True)
    signatures = {
        'CreatePipe': ([c.POINTER(w.HANDLE), c.POINTER(w.HANDLE), c.c_void_p, w.DWORD], w.BOOL),
        'CreatePseudoConsole': ([COORD, w.HANDLE, w.HANDLE, w.DWORD, c.POINTER(w.HANDLE)], c.c_long),
        'ClosePseudoConsole': ([w.HANDLE], None),
        'CloseHandle': ([w.HANDLE], w.BOOL),
        'GetStdHandle': ([w.DWORD], w.HANDLE),
        'SetStdHandle': ([w.DWORD, w.HANDLE], w.BOOL),
        'InitializeProcThreadAttributeList': ([c.c_void_p, w.DWORD, w.DWORD, c.POINTER(c.c_size_t)], w.BOOL),
        'UpdateProcThreadAttribute': ([c.c_void_p, w.DWORD, c.c_size_t, c.c_void_p, c.c_size_t, c.c_void_p, c.c_void_p], w.BOOL),
        'DeleteProcThreadAttributeList': ([c.c_void_p], None),
        'CreateProcessW': ([w.LPCWSTR, w.LPWSTR, c.c_void_p, c.c_void_p, w.BOOL, w.DWORD, c.c_void_p, w.LPCWSTR, c.POINTER(STARTUPINFOEX), c.POINTER(PROCESS_INFORMATION)], w.BOOL),
        'ReadFile': ([w.HANDLE, c.c_void_p, w.DWORD, c.POINTER(w.DWORD), c.c_void_p], w.BOOL),
        'WriteFile': ([w.HANDLE, c.c_void_p, w.DWORD, c.POINTER(w.DWORD), c.c_void_p], w.BOOL),
        'WaitForSingleObject': ([w.HANDLE, w.DWORD], w.DWORD),
        'GetExitCodeProcess': ([w.HANDLE, c.POINTER(w.DWORD)], w.BOOL),
    }
    for name, (params, result) in signatures.items():
        fn = getattr(kernel, name)
        fn.argtypes, fn.restype = params, result

    def checked(value):
        if not value:
            raise c.WinError(c.get_last_error())
        return value

    input_read, input_write, output_read, output_write, hpc = [w.HANDLE() for _ in range(5)]
    checked(kernel.CreatePipe(c.byref(input_read), c.byref(input_write), None, 0))
    checked(kernel.CreatePipe(c.byref(output_read), c.byref(output_write), None, 0))
    hr = kernel.CreatePseudoConsole(COORD(120, 40), input_read, output_write, 0, c.byref(hpc))
    if hr < 0:
        raise RuntimeError(f'CreatePseudoConsole HRESULT {hr:#x}')
    size = c.c_size_t()
    kernel.InitializeProcThreadAttributeList(None, 1, 0, c.byref(size))
    attributes = c.create_string_buffer(size.value)
    checked(kernel.InitializeProcThreadAttributeList(attributes, 1, 0, c.byref(size)))
    checked(kernel.UpdateProcThreadAttribute(attributes, 0, 0x00020016, hpc, c.sizeof(hpc), None, None))
    startup = STARTUPINFOEX()
    startup.StartupInfo.cb = c.sizeof(startup)
    startup.lpAttributeList = c.cast(attributes, c.c_void_p)
    process = PROCESS_INFORMATION()
    argv = [r'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', str(script)]
    command = c.create_unicode_buffer(subprocess.list2cmdline(argv))
    saved_handles = {identifier: kernel.GetStdHandle(identifier) for identifier in (0xfffffff6, 0xfffffff5, 0xfffffff4)}
    try:
        for identifier in saved_handles:
            checked(kernel.SetStdHandle(identifier, None))
        checked(kernel.CreateProcessW(None, command, None, None, False, 0x00080000, None, str(root), c.byref(startup), c.byref(process)))
    finally:
        for identifier, handle in saved_handles.items():
            kernel.SetStdHandle(identifier, handle)
    kernel.CloseHandle(input_read)
    kernel.CloseHandle(output_write)
    kernel.DeleteProcThreadAttributeList(attributes)
    lock = threading.Lock()
    raw_path, input_path = root / 'terminal.raw', root / 'terminal-input.json'
    if not input_path.exists():
        input_path.write_text('[]')
    provenance = {'transport': 'Windows CreatePseudoConsole', 'pid': process.dwProcessId,
                  'argv': argv, 'launchScript': str(script), 'startedAt': time.time(),
                  'dimensions': [120, 40], 'transcript': str(raw_path)}
    (root / 'conpty-provenance.json').write_text(json.dumps(provenance, indent=2))

    def send(data):
        with lock:
            done = w.DWORD()
            checked(kernel.WriteFile(input_write, data, len(data), c.byref(done), None))

    def read_output():
        with raw_path.open('wb', buffering=0) as output:
            while True:
                buffer, count = c.create_string_buffer(65536), w.DWORD()
                if not kernel.ReadFile(output_read, buffer, len(buffer), c.byref(count), None) or not count.value:
                    break
                chunk = buffer.raw[:count.value]
                output.write(chunk)
                if b'\x1b[6n' in chunk:
                    send(b'\x1b[1;1R')

    reader = threading.Thread(target=read_output, daemon=True)
    reader.start()
    print(json.dumps({'conptyReady': True, 'root': str(root), 'pid': process.dwProcessId}), flush=True)
    seen, deadline = 0, time.monotonic() + args.timeout
    try:
        while kernel.WaitForSingleObject(process.hProcess, 100) == 258:
            if time.monotonic() > deadline:
                provenance['timedOut'] = True
                cleanup = subprocess.run(['taskkill', '/PID', str(process.dwProcessId), '/T', '/F'], capture_output=True, timeout=10)
                provenance['timeoutTreeCleanupExitCode'] = cleanup.returncode
                break
            try:
                entries = json.loads(input_path.read_text(encoding='utf-8-sig'))
            except (ValueError, OSError):
                continue
            for entry in entries[seen:]:
                send(entry.encode('utf-8'))
            seen = len(entries)
    finally:
        kernel.ClosePseudoConsole(hpc)
        reader.join(5)
        code = w.DWORD()
        kernel.GetExitCodeProcess(process.hProcess, c.byref(code))
        provenance.update(exitCode=code.value, finishedAt=time.time(), readerFinished=not reader.is_alive())
        (root / 'conpty-provenance.json').write_text(json.dumps(provenance, indent=2))
        for handle in [input_write, output_read, process.hThread, process.hProcess]:
            kernel.CloseHandle(handle)
        print(json.dumps({'conptyFinished': True, **provenance}), flush=True)


if __name__ == '__main__':
    main()
