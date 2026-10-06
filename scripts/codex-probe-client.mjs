import { spawn, execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { basename, join } from 'node:path';
import { homedir } from 'node:os';

export const codexHome = process.env.CODEX_HOME || join(homedir(), '.codex');

export function daemonExecutable() {
  const daemon = JSON.parse(readFileSync(join(codexHome, 'app-server-daemon/daemon.pid'), 'utf8'));
  if (!Number.isSafeInteger(daemon.pid) || daemon.pid <= 0 || !/^\d+$/.test(daemon.processStartTime)) {
    throw new Error('Invalid daemon identity');
  }
  const code = `$taskProcess = Get-Process -Id ${daemon.pid} -ErrorAction Stop; if ($taskProcess.StartTime.ToFileTimeUtc().ToString() -ne '${daemon.processStartTime}') { throw 'Daemon identity changed' }; $taskProcess.Path`;
  const path = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', code], {
    encoding: 'utf8', windowsHide: true, timeout: 10000,
  }).trim();
  if (basename(path).toLowerCase() !== 'codex.exe') throw new Error('Unexpected daemon executable');
  return path;
}

export async function connect(executable) {
  let child;
  let pipe;
  const pending = new Map();
  const events = [];
  let nextId = 0;
  let wake;
  const server = createServer(socket => {
    if (pipe) return socket.destroy();
    pipe = socket;
    child = spawn(executable, ['app-server', 'proxy', '--sock', join(codexHome, 'app-server-control/app-server-control.sock')], {
      stdio: ['pipe', 'pipe', 'ignore'], windowsHide: true,
    });
    socket.pipe(child.stdin);
    child.stdout.pipe(socket);
    child.stdin.on('error', () => socket.destroy());
    child.on('error', () => socket.destroy());
    child.on('exit', () => socket.destroy());
    socket.on('error', () => {});
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const socket = new WebSocket(`ws://127.0.0.1:${server.address().port}/`);
  async function close() {
    socket.close();
    pipe?.destroy();
    if (child && child.exitCode === null) {
      const exited = new Promise(resolve => child.once('exit', resolve));
      child.kill();
      await exited;
    }
    await new Promise(resolve => server.close(resolve));
  }
  socket.addEventListener('message', event => {
    const value = JSON.parse(event.data);
    const task = !value.method && pending.get(value.id);
    if (task) {
      pending.delete(value.id);
      clearTimeout(task.timer);
      if (value.error) task.reject(new Error(value.error.message));
      else task.resolve(value.result);
    } else {
      events.push(value);
      wake?.();
    }
  });
  socket.addEventListener('close', () => {
    for (const task of pending.values()) {
      clearTimeout(task.timer);
      task.reject(new Error('Codex proxy disconnected'));
    }
    pending.clear();
    wake?.();
  });
  try {
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('Proxy connection timed out')), 10000);
      socket.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
      socket.addEventListener('error', () => { clearTimeout(timer); reject(new Error('Proxy connection failed')); }, { once: true });
    });
    const send = value => socket.send(JSON.stringify(value));
    function request(method, params) {
      const id = ++nextId;
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          pending.delete(id);
          reject(new Error(`${method} timed out`));
        }, 15000);
        pending.set(id, { resolve, reject, timer });
        send({ id, method, params });
      });
    }
    async function event(predicate, timeout = 90000) {
      const deadline = Date.now() + timeout;
      while (Date.now() < deadline) {
        const index = events.findIndex(predicate);
        if (index >= 0) return events.splice(index, 1)[0];
        if (socket.readyState !== WebSocket.OPEN) throw new Error('Proxy closed while waiting');
        await new Promise(resolve => {
          const timer = setTimeout(resolve, Math.min(1000, deadline - Date.now()));
          wake = () => { clearTimeout(timer); resolve(); };
        });
        wake = undefined;
      }
      throw new Error('Expected Codex event timed out');
    }
    await request('initialize', {
      clientInfo: { name: 'capy_waiting_verifier', version: '0.1.0' },
      capabilities: { experimentalApi: true },
    });
    send({ method: 'initialized', params: {} });
    return { request, event, send, close };
  } catch (error) {
    await close();
    throw error;
  }
}
