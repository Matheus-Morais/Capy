import type { QuotaRow } from './bridge';

const html = (value: string) => value.replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));

export function usableQuota(q: QuotaRow, now: number): boolean {
  const w = q.window;
  const age = q.observedAt === null ? -1 : now - q.observedAt;
  return !!(q.state === 'fresh' && q.account && w && Number.isFinite(w.usedPercent)
    && w.usedPercent >= 0 && w.usedPercent <= 100 && Number.isFinite(w.windowDurationMins)
    && w.windowDurationMins > 0 && Number.isFinite(w.resetsAt) && Number.isFinite(age)
    && age >= 0 && age <= 120_000 && w.resetsAt * 1000 > now && w.resetsAt * 1000 <= 8.64e15);
}

export function resetCountdown(reset: number, now: number): string {
  const minutes = Math.max(1, Math.ceil((reset * 1000 - now) / 60_000));
  const days = Math.floor(minutes / 1440), hours = Math.floor(minutes % 1440 / 60), mins = minutes % 60;
  return days ? `${days}d ${hours}h` : hours ? `${hours}h ${mins}min` : `${mins}min`;
}

function windowLabel(q: QuotaRow): string {
  if (q.period === 'five_hour' || q.window?.windowDurationMins === 300) return '5 horas';
  if (q.period === 'seven_day' || q.window?.windowDurationMins === 10080) return 'Semanal';
  return q.period === 'secondary' ? 'Janela secundária' : 'Janela principal';
}

function meter(q: QuotaRow, now: number): string {
  const label = windowLabel(q);
  const valid = usableQuota(q, now);
  const remaining = valid ? Number((100 - q.window!.usedPercent).toFixed(1)) : null;
  const expired = q.state === 'stale' || (q.state === 'fresh' && !valid);
  const status = expired ? 'Dado expirado' : 'Indisponível';
  const date = valid ? new Date(q.window!.resetsAt * 1000).toLocaleString('pt-BR', {dateStyle:'short',timeStyle:'short'}) : '';
  const observed = q.observedAt !== null && Number.isFinite(q.observedAt) && Math.abs(q.observedAt) <= 8.64e15
    ? new Date(q.observedAt).toLocaleTimeString('pt-BR', {hour:'2-digit',minute:'2-digit',second:'2-digit'}) : null;
  return `<section class="quota-window${remaining !== null && remaining <= 20 ? ' quota-low' : ''}"><h4>${html(label)}</h4>
    <div class="quota-dial${valid ? '' : ' quota-dial-empty'}"${valid ? ` role="meter" aria-label="${html(`${q.provider} · ${q.account} · ${q.bucket ?? ''} · ${label} disponível`)}" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${remaining}" style="--remaining:${remaining}"` : ''}>
      <span>${remaining === null ? '—' : `${remaining}<small>%</small>`}</span></div>
    <p class="quota-balance">${valid ? 'disponível' : status}</p>
    ${valid ? `<p class="quota-reset">Renova em <strong>${resetCountdown(q.window!.resetsAt, now)}</strong></p>` : ''}
    <details class="quota-detail" data-quota-key="${html(JSON.stringify([q.provider,q.account,q.bucket,q.period]))}"><summary>Fonte e horário</summary>${valid?`<p>Renovação: ${html(date)}</p>`:''}<p>${observed ? `${valid ? 'Observado' : 'Última observação'} às ${html(observed)}` : 'Sem observação confirmada'}</p><p>${html(q.message)}</p></details></section>`;
}

export function quotaBoard(real: boolean, rows: QuotaRow[] = [], now = Date.now()): string {
  if (!real) rows = [
    ['Claude','Conta pessoal',38,72], ['Claude','Conta trabalho',84,56], ['Codex','Conta pessoal',72,61],
  ].flatMap(([provider,account,five,week]) => [300,10080].map((duration,i) => ({
    provider: String(provider),account:String(account),bucket:null,period:i?'seven_day':'five_hour',
    window:{usedPercent:100-Number(i?week:five),windowDurationMins:duration,resetsAt:now/1000+(i?86400:7200)},
    state:'fresh',observedAt:now,message:'Exemplo simulado. Não representa o saldo de uma conta real.',
  })));
  if (!rows.length) return '<p class="quota-unavailable">Nenhuma cota confirmada ainda. Conecte uma conta Claude ou abra o Codex para acompanhar o saldo.</p>';
  const groups = new Map<string, QuotaRow[]>();
  for (const row of rows) {
    const key = JSON.stringify([row.provider, row.account]);
    const group = groups.get(key) ?? []; group.push(row); groups.set(key,group);
  }
  return [...groups.values()].map(group => {
    const first = group[0];
    const fresh = group.filter(q => usableQuota(q,now));
    const low = fresh.some(q => q.window!.usedPercent >= 80);
    const status = !fresh.length ? 'Saldo indisponível' : low ? 'Saldo baixo' : 'Saldo atualizado';
    const buckets = new Map<string, QuotaRow[]>();
    for (const row of group) { const key=row.bucket??''; const bucket=buckets.get(key)??[]; bucket.push(row); buckets.set(key,bucket); }
    return `<article class="quota-account"><header><div><h3>${html(first.provider)}</h3><p>${html(first.account ?? 'Conta não confirmada')}</p></div><span class="quota-account-state${low?' quota-low':!fresh.length?' quota-unknown':''}">${status}</span></header>
      ${[...buckets.entries()].map(([bucket,windows])=>`<div class="quota-bucket">${bucket?`<p class="quota-bucket-label">${html(bucket)}</p>`:''}<div class="quota-windows">${windows.map(q=>meter(q,now)).join('')}</div></div>`).join('')}</article>`;
  }).join('');
}

export function renderQuotaBoard(element: HTMLElement, real: boolean, rows: QuotaRow[], now = Date.now()): () => void {
  let timer: ReturnType<typeof setTimeout>;
  function draw(time: number) {
    const disclosures = [...element.querySelectorAll<HTMLDetailsElement>('details')];
    const open = new Set(disclosures.filter(detail=>detail.open).map(detail=>detail.dataset.quotaKey));
    const focus = disclosures.find(detail => detail.querySelector('summary') === document.activeElement)?.dataset.quotaKey;
    element.innerHTML = quotaBoard(real, rows, time);
    element.querySelectorAll<HTMLDetailsElement>('details').forEach(detail => {
      detail.open = open.has(detail.dataset.quotaKey);
      if (focus && focus === detail.dataset.quotaKey) detail.querySelector('summary')?.focus({preventScroll:true});
    });
    const deadlines = real ? rows.filter(q => usableQuota(q,time)).flatMap(q => [q.observedAt! + 120_001,q.window!.resetsAt * 1000]) : [];
    const delay = Math.min(60_000 - time % 60_000, ...deadlines.map(d => d-time));
    timer = setTimeout(() => draw(Date.now()), Math.max(1,delay));
  }
  draw(now);
  return () => clearTimeout(timer);
}
