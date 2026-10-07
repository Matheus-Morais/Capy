import { savePreferences, dismissQuotaAlert, defaultPreferences, showError, type Snapshot, type Preferences, type QuotaRule, type AccountProfile } from './bridge';
import { escape, defaultThresholds, ruleMarkup, fallbackMarkup } from './presentation';

let preferences: Preferences = defaultPreferences();
let rules: QuotaRule[] = [];
let signature = '\0';
let initialized = false;
let profiles:AccountProfile[]=[];
let latest:Snapshot|undefined;
function initialize() {
  const controls = document.querySelector('.demo-controls');
  if (!controls || initialized) return;
  initialized = true;
  window.addEventListener('capy-profiles',event=>{
    const values=(event as CustomEvent<AccountProfile[]>).detail;
    if(JSON.stringify(values)!==JSON.stringify(profiles)){profiles=values;signature='';if(latest)renderPreferences(latest);}
  });
  controls.insertAdjacentHTML('beforeend', '<label><input id="petSounds" type="checkbox"> Sons da Capy</label><p>Sinais curtos para pedidos e conclusões. Desligados por padrão.</p>');
  document.getElementById('petSounds')!.addEventListener('change', event => {
    void savePreferences({...preferences, sounds:(event.target as HTMLInputElement).checked}).catch(showError);
  });
  controls.insertAdjacentHTML('afterend', '<section class="preferences" id="quotaPreferences"><h2>Alertas por conta</h2><p>Cada percentual avisa uma vez por janela. Se vários forem ultrapassados juntos, mostramos o maior.</p><div id="quotaRuleForms"></div></section>');
  document.getElementById('quotaRuleForms')!.addEventListener('click', event => {
    const destination=(event.target as Element).closest('[data-add-destination]');
    if(destination){destination.closest('form')!.querySelector('.fallback-chain')!.insertAdjacentHTML('beforeend',fallbackMarkup(profiles));return;}
    const chain=(event.target as Element).closest<HTMLButtonElement>('[data-chain]');
    if(chain){
      const row=chain.closest<HTMLElement>('.fallback-row')!;
      if(chain.dataset.chain==='remove')row.remove();
      else if(chain.dataset.chain==='up'&&row.previousElementSibling)row.previousElementSibling.before(row);
      else if(chain.dataset.chain==='down'&&row.nextElementSibling)row.nextElementSibling.after(row);
      return;
    }
    const add = (event.target as Element).closest('[data-add-threshold]');
    if (!add) return;
    const form = add.closest('form')!;
    const existing = [...form.querySelectorAll<HTMLInputElement>('[data-percent]')].map(i=>Number(i.value));
    const next = [95,100,55,65,75,85].find(p=>!existing.includes(p)) ?? Array.from({length:100},(_,i)=>i+1).find(p=>!existing.includes(p));
    if (next === undefined) return;
    form.querySelector('.thresholds')!.insertAdjacentHTML('beforeend', `<label><input type="checkbox" data-enabled checked><input type="number" data-percent min="1" max="100" required value="${next}" aria-label="Percentual do alerta">%</label>`);
  });
  document.getElementById('quotaRuleForms')!.addEventListener('submit', event => {
    event.preventDefault();
    const form = event.target as HTMLFormElement;
    if (!form.reportValidity()) return;
    const rule = rules[Number(form.dataset.ruleIndex)];
    const thresholds = [...form.querySelectorAll<HTMLLabelElement>('.thresholds label')].map(label=>({percent:Number(label.querySelector<HTMLInputElement>('[data-percent]')!.value),enabled:label.querySelector<HTMLInputElement>('[data-enabled]')!.checked}));
    const trigger=(name:string)=>{const value=form.querySelector<HTMLInputElement>(`[data-trigger="${name}"]`)!.value;return value===''?null:Number(value);};
    const fallback=[...form.querySelectorAll<HTMLElement>('.fallback-row')].map(row=>({profileId:row.querySelector<HTMLSelectElement>('[data-destination-profile]')!.value,model:row.querySelector<HTMLInputElement>('[data-destination-model]')!.value.trim()}));
    const updated = {...rule, thresholds,fiveHourTrigger:trigger('five'),weeklyTrigger:trigger('weekly'),fallback};
    const quotaRules = preferences.quotaRules.filter(r=>!(r.provider===rule.provider && r.account===rule.account));
    quotaRules.push(updated);
    const button = form.querySelector<HTMLButtonElement>('[type="submit"]')!;
    button.disabled = true;
    void savePreferences({...preferences,quotaRules}).then(()=> {document.getElementById('notice')!.textContent='Alertas salvos para esta conta.';}).catch(showError).finally(()=>button.disabled=false);
  });
}

export function renderPreferences(data: Snapshot): void {
  latest=data;
  initialize();
  preferences = data.preferences ?? {...defaultPreferences(), reduceMotion:data.reduceMotion};
  const sound = document.getElementById('petSounds') as HTMLInputElement | null;
  if (sound) sound.checked = preferences.sounds;
  const forms = document.getElementById('quotaRuleForms');
  if (forms) {
    const accounts = new Map<string, {provider:string; account:string}>();
    for (const q of [...(data.quotas ?? []),...preferences.quotaRules]) if (q.account) accounts.set(JSON.stringify([q.provider,q.account]),{provider:q.provider,account:q.account});
    rules = [...accounts.values()].map(q=>preferences.quotaRules.find(r=>r.provider===q.provider && r.account===q.account) ?? {...q,thresholds:defaultThresholds(),fiveHourTrigger:null,weeklyTrigger:null,fallback:[]});
    const next = JSON.stringify(rules);
    if (signature !== next) {
      signature = next;
      forms.innerHTML = rules.length ? rules.map((rule,index)=>ruleMarkup(rule,index,profiles)).join('') : '<p>As contas aparecem quando a integração confirmar sua identidade.</p>';
    }
  }
  let alerts = document.getElementById('quotaAlerts');
  if (!alerts) {
    alerts = document.createElement('section');
    alerts.id = 'quotaAlerts'; alerts.className='quota-alerts'; alerts.setAttribute('aria-label','Avisos de quota'); alerts.setAttribute('aria-live','polite');
    document.querySelector('.simulation')?.after(alerts);
    alerts.addEventListener('click',event=>{
      const button=(event.target as Element).closest<HTMLButtonElement>('[data-dismiss-alert]');
      if (button) void dismissQuotaAlert(button.dataset.dismissAlert!).catch(showError);
    });
  }
  const html=(data.quotaAlerts ?? []).filter(a=>a.resetsAt*1000>Date.now()).map(a=>`<p><span><strong>${escape(a.provider)} · ${escape(a.account)}</strong><br>Uso passou de ${a.percent}% · ${escape(a.period==='five_hour'?'5 horas':a.period==='seven_day'?'semanal':a.period==='primary'?'janela principal':'janela secundária')}</span><button data-dismiss-alert="${escape(a.id)}" aria-label="Dispensar aviso de ${a.percent}%">Dispensar</button></p>`).join('')+(data.routing??[]).map(r=>`<p><span>${escape(r.message)}<br>Sessão ${escape(r.taskId)}</span></p>`).join('');
  if (alerts.innerHTML!==html) alerts.innerHTML=html;
  alerts.hidden=!html;
}
