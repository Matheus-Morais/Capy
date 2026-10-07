import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import fixture from '../assets/demo.json';

export interface Session { id: string; project: string; agent: string; symbol: string; kind: string; origin: string; state: string; request: string | null; message: string; command: string | null; hidden: boolean; completion?: string | null; source_action?: { label: string; available: boolean; reason: string | null } }
export interface Integration { agent: string; message: string }
export interface QuotaRow { provider: string; account: string | null; bucket: string | null; period: string | null; window: { usedPercent: number; windowDurationMins: number; resetsAt: number } | null; observedAt: number | null; state: string; message: string }
export interface InterventionQuestion { id: string; header: string; question: string; options: Array<{ label: string; description: string }>; isOther: boolean; isSecret: boolean }
export interface Intervention { nonce: string; generation: string; sessionId: string; threadId: string; turnId: string; itemId: string; status: string; body: { kind: string; questions?: InterventionQuestion[]; command?: string; cwd?: string; reason?: string; changes?: unknown }; decisions: string[] }
export interface Subscription { sessionId: string; status: string; message: string }
export interface QuotaRule { provider: string; account: string; thresholds: Array<{percent: number; enabled: boolean}>; fiveHourTrigger: number | null; weeklyTrigger: number | null; fallback: Array<{profileId: string; model: string}> }
export interface Preferences { sounds: boolean; reduceMotion: boolean; quotaRules: QuotaRule[] }
export interface QuotaAlert { id: string; provider: string; account: string; period: string; percent: number; resetsAt: number; createdAt: number }
export interface AccountProfile { id: string; label: string; provider: string; configDir: string; billing: string }
export interface AccountIdentity { loggedIn: boolean; account: string | null; billing: string; message: string }
export interface HandoffSummary { objective:string; decisions:string; state:string; files:string; tests:string; nextSteps:string; guides:string }
export interface HandoffReview { nonce:string; sourceTaskId:string; destinationProfileId:string; model:string; sourceBilling:string; destinationBilling:string; destinationAccount:string|null; summary:HandoffSummary; automatic:boolean }
export interface Snapshot { sessions: Session[]; scenario: string; reduceMotion: boolean; integrations: Integration[]; quotas?: QuotaRow[]; interventions: Intervention[]; subscriptions: Subscription[]; preferences?: Preferences; quotaAlerts?: QuotaAlert[]; handoffs?:HandoffReview[]; chatTransfers?:ChatTransferReview[]; routing?:Array<{taskId:string;state:string;message:string}> }
export const defaultPreferences = (): Preferences => ({sounds: false, reduceMotion: false, quotaRules: []});
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
export async function savePreferences(value: Preferences): Promise<void> {
  if (native) { await invoke('save_preferences', { value }); return; }
  browserSnapshot.preferences = structuredClone(value);
  browserSnapshot.reduceMotion = value.reduceMotion;
  listeners.forEach(callback => callback(structuredClone(browserSnapshot)));
}
export async function dismissQuotaAlert(id: string): Promise<void> {
  if (native) await invoke('dismiss_quota_alert', {id});
}
export async function listProfiles(): Promise<AccountProfile[]> { return native ? invoke('list_profiles') : []; }
export interface ApiAccount {
  account: {id: string; label: string; provider: 'OpenAI' | 'Anthropic' | 'Gemini'; revision: string};
  configured: boolean;
  billing: 'api';
}
export interface ChatTarget {
  kind: 'claudeCli' | 'api'; profileId: string; provider: string; account: string;
  billing: 'subscription' | 'api'; credentialRevision: string | null;
}
export interface ChatConversation {
  id: string; title: string; target: ChatTarget; model: string;
  messages: {role: 'user' | 'assistant'; text: string}[];
  revision: number; state: 'idle' | 'working' | 'completed' | 'partial' | 'failed' | 'unknown' | 'transferred';
  activeNonce: string | null; usedNonces: string[]; lastError: string | null;
  cliStarted: boolean; cliAttempted: boolean; transferredTo: string | null;
  processPolicy: string | null;
  recoveryReview: {id:string;nonce:string;revision:number;target:ChatTarget;resumeCli:boolean} | null;
  interruptions: {messageIndex:number;sendNonce:string;approvalNonce:string}[];
}
export async function prepareChatRecovery(id:string,revision:number):Promise<ChatConversation>{
  if(!native)throw new Error('Recuperação disponível no aplicativo desktop.');
  return invoke('prepare_chat_recovery',{id,revision});
}
export async function approveChatRecovery(id:string,revision:number,nonce:string,reviewed:boolean):Promise<ChatConversation>{
  if(!native)throw new Error('Recuperação disponível no aplicativo desktop.');
  return invoke('approve_chat_recovery',{id,revision,nonce,reviewed});
}
export interface CreateChat {
  title: string; kind: ChatTarget['kind']; profileId: string; model: string;
  expectedAccount: string; expectedBilling: ChatTarget['billing']; credentialRevision: string | null;
}
export interface ChatTransferReview {
  nonce:string; sourceId:string; sourceRevision:number; sourceTarget:ChatTarget;
  destination:ChatTarget; model:string; summary:HandoffSummary;
  uncertainMessages?:number[];
}
export async function chatTransferReview(sourceId:string):Promise<ChatTransferReview|null>{return native?invoke('chat_transfer_review',{sourceId}):null;}
export async function prepareChatTransfer(sourceId:string,request:CreateChat):Promise<ChatTransferReview>{
  if(!native)throw new Error('Transferência disponível no aplicativo desktop.');
  return invoke('prepare_chat_transfer',{sourceId,request});
}
export async function approveChatTransfer(sourceId:string,nonce:string,summary:HandoffSummary,reviewed:boolean,billingConfirmed:boolean):Promise<ChatConversation>{
  if(!native)throw new Error('Transferência disponível no aplicativo desktop.');
  return invoke('approve_chat_transfer',{sourceId,nonce,summary,reviewed,billingConfirmed});
}
export async function cancelChatTransfer(sourceId:string,nonce:string):Promise<void>{
  if(!native)throw new Error('Transferência disponível no aplicativo desktop.');
  return invoke('cancel_chat_transfer',{sourceId,nonce});
}
export async function listApiAccounts(): Promise<ApiAccount[]> { return native ? invoke('list_api_accounts') : []; }
export async function addApiAccount(label: string, provider: ApiAccount['account']['provider'], key: string): Promise<ApiAccount> {
  if (!native) throw new Error('Chaves API disponíveis no aplicativo desktop.');
  return invoke('add_api_account', {label, provider, key});
}
export async function listChats(): Promise<ChatConversation[]> { return native ? invoke('list_chats') : []; }
export async function createChat(request: CreateChat): Promise<ChatConversation> {
  if (!native) throw new Error('Chat disponível no aplicativo desktop.');
  return invoke('create_chat', {request});
}
export async function sendChat(request: {id: string; revision: number; nonce: string; model: string; text: string}): Promise<ChatConversation> {
  if (!native) throw new Error('Chat disponível no aplicativo desktop.');
  return invoke('send_chat', {request});
}
export async function addProfile(label: string, existing: string | null, billing: string): Promise<AccountProfile> {
  if (!native) throw new Error('Cadastro de contas disponível no aplicativo desktop.');
  return invoke('add_profile', {label, existing, billing});
}
export async function profileIdentity(id: string, cwd: string | null = null): Promise<AccountIdentity> { return invoke('profile_identity', {id,cwd}); }
export async function loginProfile(id: string): Promise<void> { return invoke('login_profile', {id}); }
export async function connectClaudeQuotas(id: string, enabled: boolean): Promise<void> { return invoke('connect_claude_quotas', {id,enabled}); }
export function showError(error: unknown): void {
  const element = document.getElementById('error');
  if (element) { element.hidden = false; element.textContent = `Não foi possível concluir: ${String(error)}. Tente novamente.`; }
  if (native) void invoke('ui_error', { message: String(error).slice(0, 500) }).catch(() => {});
}
