import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import fixture from '../assets/demo.json';

export interface Session { id: string; project: string; agent: string; symbol: string; kind: string; origin: string; state: string; request: string | null; message: string; command: string | null; hidden: boolean; source_action?: { label: string; available: boolean; reason: string | null } }
export interface Integration { agent: string; message: string }
export interface QuotaRow { provider: string; account: string | null; bucket: string | null; period: string | null; window: { usedPercent: number; windowDurationMins: number; resetsAt: number } | null; observedAt: number | null; state: string; message: string }
export interface InterventionQuestion { id: string; header: string; question: string; options: Array<{ label: string; description: string }>; isOther: boolean; isSecret: boolean }
export interface Intervention { nonce: string; generation: string; sessionId: string; threadId: string; turnId: string; itemId: string; status: string; body: { kind: string; questions?: InterventionQuestion[]; command?: string; cwd?: string; reason?: string; changes?: unknown }; decisions: string[] }
export interface Subscription { sessionId: string; status: string; message: string }
export interface Snapshot { sessions: Session[]; scenario: string; reduceMotion: boolean; integrations: Integration[]; quotas?: QuotaRow[]; interventions: Intervention[]; subscriptions: Subscription[] }
export const native = isTauri();
let browserSnapshot: Snapshot = { sessions: structuredClone(fixture), scenario: 'waiting', reduceMotion: false, integrations: [], interventions: [], subscriptions: [] };
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
    if (answer === 'real') throw new Error('A descoberta real está disponível no aplicativo desktop.');
    browserSnapshot.scenario = answer;
    browserSnapshot.sessions = answer === 'sleeping' ? [] : structuredClone(fixture);
    browserSnapshot.interventions = [];
    browserSnapshot.subscriptions = [];
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
export async function respondIntervention(context: Omit<Intervention, 'status' | 'body' | 'decisions'>, response: Record<string, unknown>): Promise<void> {
  if (native) await invoke('respond_intervention', { context, response });
}
export async function setInterventionSubscription(id: string, enabled: boolean): Promise<void> {
  if (native) await invoke('connect_interventions', { id, enabled });
}
export function showError(error: unknown): void {
  const element = document.getElementById('error');
  if (element) { element.hidden = false; element.textContent = `Não foi possível concluir: ${String(error)}. Tente novamente.`; }
  if (native) void invoke('ui_error', { message: String(error).slice(0, 500) }).catch(() => {});
}
