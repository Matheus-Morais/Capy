import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import fixture from '../assets/demo.json';

export interface Session { id: string; project: string; agent: string; symbol: string; kind: string; origin: string; state: string; request: string | null; message: string; command: string | null; hidden: boolean }
export interface Snapshot { sessions: Session[]; scenario: string; reduceMotion: boolean }
export const native = isTauri();
let browserSnapshot: Snapshot = { sessions: structuredClone(fixture), scenario: 'waiting', reduceMotion: false };
const listeners: Array<(value: Snapshot) => void> = [];

export async function snapshot(): Promise<Snapshot> {
  return native ? invoke('demo_snapshot') : structuredClone(browserSnapshot);
}
export async function subscribe(callback: (data: Snapshot) => void): Promise<void> {
  if (native) await listen<Snapshot>('demo-updated', event => callback(event.payload));
  else listeners.push(callback);
}
export async function subscribeVisibility(callback: (open: boolean) => void): Promise<void> {
  if (native) await listen<boolean>('summary-visibility', event => callback(event.payload));
}
export async function action(action: string, id = '', answer = ''): Promise<void> {
  if (native) { await invoke('demo_action', { action, id, answer }); return; }
  if (action === 'scenario') {
    browserSnapshot.scenario = answer;
    browserSnapshot.sessions = answer === 'sleeping' ? [] : structuredClone(fixture);
    if (answer !== 'waiting') browserSnapshot.sessions.forEach(s => { s.state = answer; s.message = answer === 'working' ? 'O agente está executando sua tarefa.' : 'A tarefa foi concluída. Confira o resultado no terminal.'; });
  } else if (action === 'motion') browserSnapshot.reduceMotion = answer === 'true';
  else if (action === 'restore') browserSnapshot.sessions.forEach(s => s.hidden = false);
  else {
    const s = browserSnapshot.sessions.find(s => s.id === id);
    if (!s) throw new Error('Sessão não encontrada.');
    if (action === 'hide') s.hidden = true;
    else { s.state = 'working'; s.message = action === 'allow' ? 'Permissão simulada concedida. Executando os testes.' : action === 'deny' ? 'Permissão simulada negada. Procurando outra alternativa.' : `Resposta simulada: ${answer}. O agente está continuando.`; }
  }
  listeners.forEach(callback => callback(structuredClone(browserSnapshot)));
}
export async function desktopCommand(command: string, args?: Record<string, unknown>): Promise<void> {
  if (native) { await invoke(command, args); return; }
  if (command === 'toggle_summary') window.location.assign('./summary.html');
  else if (command === 'show_panel') window.location.assign('./panel.html');
  else if (command === 'hide_window') window.location.assign('./index.html');
}
export function showError(error: unknown): void {
  const element = document.getElementById('error');
  if (element) { element.hidden = false; element.textContent = `Não foi possível concluir: ${String(error)}. Tente novamente.`; }
  if (native) void invoke('ui_error', { message: String(error).slice(0, 500) }).catch(() => {});
}
