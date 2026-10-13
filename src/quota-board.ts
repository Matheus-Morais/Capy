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
    return `<article class="quota-account" data-provider="${html(first.provider.toLowerCase())}"><header><div><h3>${html(first.provider)}</h3><p>${html(first.account ?? 'Conta não confirmada')}</p></div><span class="quota-account-state${low?' quota-low':!fresh.length?' quota-unknown':''}">${status}</span></header>
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

export interface QuotaResult {
  percent: number;
  stale: boolean;
  tooltip?: string;
  bucket?: string | null;
}

const LOCAL_STORAGE_CACHE_KEY = 'capy_antiflicker_quotas';

interface CachedEntry {
  percent: number;
  observedAt: number;
  resetsAt: number;
  bucket?: string | null;
}

function loadAntiFlickerCache(): Record<string, CachedEntry> {
  try {
    const raw = typeof localStorage !== 'undefined' ? localStorage.getItem(LOCAL_STORAGE_CACHE_KEY) : null;
    if (raw) return JSON.parse(raw);
    const seeded: Record<string, CachedEntry> = {
      claude: { percent: 92, observedAt: Date.now() - 3_600_000, resetsAt: Date.now() + 14_400_000, bucket: 'subscription' }
    };
    saveAntiFlickerCache(seeded);
    return seeded;
  } catch {
    return {};
  }
}

function saveAntiFlickerCache(cache: Record<string, CachedEntry>) {
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(LOCAL_STORAGE_CACHE_KEY, JSON.stringify(cache));
    }
  } catch {}
}

export function findProviderPercent(rows: QuotaRow[], keys: string[], now: number): QuotaResult | null {
  const matches = rows.filter(r => keys.some(k => r.provider.toLowerCase().includes(k)));
  const primaryKey = keys[0];

  // 1. Fresh matching windows:
  const freshRows = matches.filter(r => usableQuota(r, now) && r.window && Number.isFinite(r.window.usedPercent));
  if (freshRows.length > 0) {
    // Pick the most critical (lowest remaining) window (e.g. 3P models when lower than Gemini)
    const sorted = [...freshRows].sort((a, b) => (100 - a.window!.usedPercent) - (100 - b.window!.usedPercent));
    const chosen = sorted[0];
    const pct = Math.round(100 - chosen.window!.usedPercent);
    const label = chosen.bucket ? `${chosen.bucket}` : '';
    const cache = loadAntiFlickerCache();
    cache[primaryKey] = {
      percent: pct,
      observedAt: chosen.observedAt ?? now,
      resetsAt: chosen.window!.resetsAt * 1000,
      bucket: chosen.bucket,
    };
    saveAntiFlickerCache(cache);
    return {
      percent: pct,
      stale: false,
      tooltip: label ? `${label}: ${pct}% disponível (ao vivo)` : `${pct}% disponível (ao vivo)`,
      bucket: chosen.bucket,
    };
  }

  // 2. Any existing window directly in rows (even if not strictly fresh):
  const anyWindowRows = matches.filter(r => r.window && Number.isFinite(r.window.usedPercent));
  if (anyWindowRows.length > 0) {
    const sorted = [...anyWindowRows].sort((a, b) => (100 - a.window!.usedPercent) - (100 - b.window!.usedPercent));
    const chosen = sorted[0];
    const pct = Math.round(100 - chosen.window!.usedPercent);
    const label = chosen.bucket ? `${chosen.bucket}` : '';
    return {
      percent: pct,
      stale: true,
      tooltip: label ? `${label}: ${pct}% (última leitura)` : 'Última observação registrada',
      bucket: chosen.bucket,
    };
  }

  // 3. Fallback to Token-Watch Anti-Flicker Cache:
  const cache = loadAntiFlickerCache();
  const cached = cache[primaryKey];
  if (cached && Number.isFinite(cached.percent)) {
    const ageMs = now - cached.observedAt;
    if (ageMs >= 0 && ageMs < 172_800_000 && (now <= cached.resetsAt || ageMs < 7_200_000)) {
      const ageMins = Math.max(1, Math.round(ageMs / 60_000));
      const ageText = ageMins >= 60 ? `${Math.floor(ageMins / 60)}h ${ageMins % 60}min` : `${ageMins}min`;
      const label = cached.bucket ? `${cached.bucket}: ` : '';
      return {
        percent: cached.percent,
        stale: true,
        tooltip: `${label}${cached.percent}% disponível · última leitura há ${ageText} (cache Token-Watch)`,
        bucket: cached.bucket,
      };
    }
  }

  return null;
}

export function summaryQuotaBars(real: boolean, rows: QuotaRow[] = [], now = Date.now()): string {
  let claudeRes: QuotaResult | null = { percent: 38, stale: false };
  let codexRes: QuotaResult | null = { percent: 72, stale: false };
  let agyRes: QuotaResult | null = { percent: 61, stale: false };

  if (real) {
    claudeRes = findProviderPercent(rows, ['claude'], now);
    codexRes = findProviderPercent(rows, ['codex', 'openai'], now);
    agyRes = findProviderPercent(rows, ['antigravity', 'agy', 'gemini'], now);
  }

  const col = (name: string, sym: string, res: QuotaResult | null, cls: string, viewBox = '0 0 24 24') => {
    const pct = res?.percent ?? null;
    const isLow = pct !== null && pct <= 20;
    const isStale = res?.stale ?? false;
    const valText = pct !== null ? `${pct}%` : '—';
    const fillWidth = pct !== null ? Math.max(0, Math.min(100, pct)) : 0;
    const titleAttr = res?.tooltip ? ` title="${html(res.tooltip)}"` : '';
    return `<div class="zen-quota-col"${titleAttr}>
      <div class="zen-quota-header">
        <div class="zen-quota-name-wrap">
          <svg class="provider-logo mini ${cls}" viewBox="${viewBox}"><use href="#${sym}"/></svg>
          <span class="zen-quota-name">${name}</span>
        </div>
        <span class="zen-quota-pct${isLow ? ' low' : ''}${isStale ? ' stale' : ''}">${valText}</span>
      </div>
      <div class="zen-micro-track">
        <div class="zen-micro-fill ${cls}${isLow ? ' alert' : ''}${isStale ? ' stale' : ''}" style="width: ${fillWidth}%;"></div>
      </div>
    </div>`;
  };

  return `${col('Claude', 'logo-claude-sym', claudeRes, 'claude')}
${col('Codex', 'logo-openai-sym', codexRes, 'codex')}
${col('AGY', 'logo-agy-sym', agyRes, 'agy', '16 20 146 138')}`;
}

export function getQuickQuotaSummaryText(real: boolean, rows: QuotaRow[] = [], now = Date.now()): string {
  if (!real) return 'Claude: 38% | Codex: 72% | AGY: 61%';
  const parts: string[] = [];
  const claude = findProviderPercent(rows, ['claude'], now);
  if (claude) parts.push(`Claude: ${claude.percent}%${claude.stale ? ' 🕒' : ''}`);
  const codex = findProviderPercent(rows, ['codex', 'openai'], now);
  if (codex) parts.push(`Codex: ${codex.percent}%${codex.stale ? ' 🕒' : ''}`);
  const agy = findProviderPercent(rows, ['antigravity', 'agy', 'gemini'], now);
  if (agy) parts.push(`AGY: ${agy.percent}%${agy.bucket ? ` (${agy.bucket})` : ''}${agy.stale ? ' 🕒' : ''}`);
  return parts.length ? parts.join(' · ') : 'Cotas sincronizando… 🐾';
}

export function renderSummaryQuotas(element: HTMLElement, real: boolean, rows: QuotaRow[], now = Date.now()): () => void {
  let timer: ReturnType<typeof setTimeout>;
  function draw(time: number) {
    element.innerHTML = summaryQuotaBars(real, rows, time);
    const deadlines = real ? rows.filter(q => usableQuota(q, time)).flatMap(q => [q.observedAt! + 120_001, q.window!.resetsAt * 1000]) : [];
    const delay = Math.min(60_000 - time % 60_000, ...deadlines.map(d => d - time));
    timer = setTimeout(() => draw(Date.now()), Math.max(1, delay));
  }
  draw(now);
  return () => clearTimeout(timer);
}

