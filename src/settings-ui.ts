import { savePreferences, dismissQuotaAlert, defaultPreferences, showError, type Snapshot, type Preferences, type QuotaRule, type AccountProfile, type ApiAccount } from './bridge';
import { defaultThresholds, ruleMarkup, fallbackMarkup, quotaNotifications } from './presentation';

let preferences: Preferences = defaultPreferences();
let rules: QuotaRule[] = [];
let signature = '\0';
let initialized = false;
let profiles:AccountProfile[]=[];
let apiChoices:Pick<AccountProfile,'id'|'label'>[]=[];
const destinations=()=>[...profiles.map(p=>({id:p.id,label:`Claude CLI · ${p.label}`})),...apiChoices];
let latest:Snapshot|undefined;
function initialize() {
  const controls = document.querySelector('.demo-controls');
  if (!controls || initialized) return;
  initialized = true;
  window.addEventListener('capy-api-accounts',event=>{
    const values=(event as CustomEvent<ApiAccount[]>).detail.filter(a=>a.configured).map(a=>({id:a.account.id,label:`API ${a.account.provider} · ${a.account.label} · cobrança por uso`}));
    if(JSON.stringify(values)!==JSON.stringify(apiChoices)){apiChoices=values;signature='';if(latest)renderPreferences(latest);}
  });
  window.addEventListener('capy-profiles',event=>{
    const values=(event as CustomEvent<AccountProfile[]>).detail;
    if(JSON.stringify(values)!==JSON.stringify(profiles)){profiles=values;signature='';if(latest)renderPreferences(latest);}
  });
  controls.insertAdjacentHTML('beforeend', '<label><input id="petSounds" type="checkbox"> Sons da Capy</label><p>Sinais curtos para pedidos e conclusões. Desligados por padrão.</p>');
  document.getElementById('petSounds')!.addEventListener('change', event => {
    void savePreferences({...preferences, sounds:(event.target as HTMLInputElement).checked}).catch(showError);
  });
  const currentSkin = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_pet_skin')) || 'v5';
  controls.insertAdjacentHTML('beforeend', `<label style="display:grid!important;gap:4px;margin-top:14px!important;width:100%;box-sizing:border-box;font-size:12px;">Visual da Capy <select id="petSkin" style="width:100%;max-width:100%;box-sizing:border-box;padding:3px 6px;border-radius:5px;border:1px solid var(--button-border);background:var(--paper);color:var(--text);font-family:inherit;font-size:11.5px;"><option value="v5"${currentSkin === 'v5' ? ' selected' : ''}>v5: Expressiva (olhos grandes)</option><option value="v4"${currentSkin === 'v4' ? ' selected' : ''}>v4: Proporcional (cabeça menor)</option><option value="v3"${currentSkin === 'v3' ? ' selected' : ''}>v3: Esbelta (mais magra)</option><option value="v2"${currentSkin === 'v2' ? ' selected' : ''}>v2: Fofinha (cheinha)</option><option value="v1"${currentSkin === 'v1' ? ' selected' : ''}>v1: Ursinho Clássico (protótipo)</option></select></label><p style="margin:4px 0 0;font-size:11px;color:var(--muted);line-height:1.4;">Alterne entre as versões v5, v4, v3, v2 e v1.</p>`);
  document.getElementById('petSkin')?.addEventListener('change', event => {
    const value = (event.target as HTMLSelectElement).value;
    try {
      localStorage.setItem('capy_pet_skin', value);
      window.dispatchEvent(new StorageEvent('storage', { key: 'capy_pet_skin', newValue: value }));
    } catch {}
  });
  const currentHat = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_hat')) || 'none';
  const currentClothes = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_clothes')) || 'none';
  const currentCostume = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_costume')) || 'none';
  const currentFriend = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_friend')) || 'none';
  const currentEnv = (typeof localStorage !== 'undefined' && localStorage.getItem('capy_env')) || 'auto';
  controls.insertAdjacentHTML('beforeend', `
    <div style="margin-top:12px;padding:10px;border:1px solid var(--button-border);border-radius:8px;background:var(--paper);width:100%;box-sizing:border-box;overflow:hidden;">
      <strong style="display:block;margin-bottom:6px;font-size:12px;">👒 Guarda-Roupa & Fantasias</strong>
      <div style="display:grid;gap:6px;width:100%;box-sizing:border-box;">
        <label style="display:grid!important;gap:2px;font-size:11px;margin-top:0!important;width:100%;box-sizing:border-box;">
          Boné / Chapéu
          <select id="wardrobeHat" style="width:100%;max-width:100%;box-sizing:border-box;padding:2px 4px;border-radius:4px;border:1px solid var(--button-border);background:var(--paper);color:var(--text);font-family:inherit;font-size:11px;">
            <option value="none"${currentHat === 'none' ? ' selected' : ''}>Nenhum</option>
            <option value="cap"${currentHat === 'cap' ? ' selected' : ''}>🧢 Boné Streetwear</option>
            <option value="straw"${currentHat === 'straw' ? ' selected' : ''}>🤠 Chapéu de Palha</option>
            <option value="top"${currentHat === 'top' ? ' selected' : ''}>🎩 Cartola Chic</option>
            <option value="beanie"${currentHat === 'beanie' ? ' selected' : ''}>🧶 Gorro de Inverno</option>
            <option value="flower"${currentHat === 'flower' ? ' selected' : ''}>🌸 Coroa de Flores</option>
          </select>
        </label>
        <label style="display:grid!important;gap:2px;font-size:11px;margin-top:0!important;width:100%;box-sizing:border-box;">
          Roupa / Acessório
          <select id="wardrobeClothes" style="width:100%;max-width:100%;box-sizing:border-box;padding:2px 4px;border-radius:4px;border:1px solid var(--button-border);background:var(--paper);color:var(--text);font-family:inherit;font-size:11px;">
            <option value="none"${currentClothes === 'none' ? ' selected' : ''}>Nenhuma</option>
            <option value="scarf"${currentClothes === 'scarf' ? ' selected' : ''}>🧣 Cachecol</option>
            <option value="bowtie"${currentClothes === 'bowtie' ? ' selected' : ''}>👔 Gravata Borboleta</option>
            <option value="hoodie"${currentClothes === 'hoodie' ? ' selected' : ''}>👕 Moletom Capy</option>
            <option value="raincoat"${currentClothes === 'raincoat' ? ' selected' : ''}>🧥 Capa de Chuva</option>
            <option value="sunglasses"${currentClothes === 'sunglasses' ? ' selected' : ''}>🕶️ Óculos Escuros</option>
          </select>
        </label>
        <label style="display:grid!important;gap:2px;font-size:11px;margin-top:0!important;width:100%;box-sizing:border-box;">
          Fantasia Completa
          <select id="wardrobeCostume" style="width:100%;max-width:100%;box-sizing:border-box;padding:2px 4px;border-radius:4px;border:1px solid var(--button-border);background:var(--paper);color:var(--text);font-family:inherit;font-size:11px;">
            <option value="none"${currentCostume === 'none' ? ' selected' : ''}>Nenhuma</option>
            <option value="wizard"${currentCostume === 'wizard' ? ' selected' : ''}>🧙 Mago / Bruxo</option>
            <option value="dino"${currentCostume === 'dino' ? ' selected' : ''}>🦖 Dino-Capy</option>
            <option value="pirate"${currentCostume === 'pirate' ? ' selected' : ''}>🏴‍☠️ Pirata</option>
            <option value="detective"${currentCostume === 'detective' ? ' selected' : ''}>🕵️ Detetive</option>
            <option value="royal"${currentCostume === 'royal' ? ' selected' : ''}>👑 Rei Capivara</option>
          </select>
        </label>
      </div>
      <p style="margin:6px 0 0;font-size:10.5px;color:var(--sub);line-height:1.35;">Atalhos: <kbd>1</kbd> chapéu, <kbd>2</kbd> roupa, <kbd>3</kbd> fantasia, <kbd>0</kbd> despir.</p>
    </div>
    <div style="margin-top:10px;padding:10px;border:1px solid var(--button-border);border-radius:8px;background:var(--paper);width:100%;box-sizing:border-box;overflow:hidden;">
      <strong style="display:block;margin-bottom:6px;font-size:12px;">🦆 Amiguinhos & Clima</strong>
      <div style="display:grid;gap:6px;width:100%;box-sizing:border-box;">
        <label style="display:grid!important;gap:2px;font-size:11px;margin-top:0!important;width:100%;box-sizing:border-box;">
          Amiguinho
          <select id="pantanalFriend" style="width:100%;max-width:100%;box-sizing:border-box;padding:2px 4px;border-radius:4px;border:1px solid var(--button-border);background:var(--paper);color:var(--text);font-family:inherit;font-size:11px;">
            <option value="none"${currentFriend === 'none' ? ' selected' : ''}>Nenhum</option>
            <option value="bird"${currentFriend === 'bird' ? ' selected' : ''}>🐦 Bem-te-vi</option>
            <option value="butterfly"${currentFriend === 'butterfly' ? ' selected' : ''}>🦋 Borboleta</option>
            <option value="duck"${currentFriend === 'duck' ? ' selected' : ''}>🦆 Patinho</option>
            <option value="turtle"${currentFriend === 'turtle' ? ' selected' : ''}>🐢 Tartaruguinha</option>
            <option value="all"${currentFriend === 'all' ? ' selected' : ''}>🌟 Todos</option>
          </select>
        </label>
        <label style="display:grid!important;gap:2px;font-size:11px;margin-top:0!important;width:100%;box-sizing:border-box;">
          Ambiente & Clima
          <select id="pantanalEnv" style="width:100%;max-width:100%;box-sizing:border-box;padding:2px 4px;border-radius:4px;border:1px solid var(--button-border);background:var(--paper);color:var(--text);font-family:inherit;font-size:11px;">
            <option value="auto"${currentEnv === 'auto' ? ' selected' : ''}>⏰ Automático (Horário)</option>
            <option value="day"${currentEnv === 'day' ? ' selected' : ''}>☀️ Ensolarado</option>
            <option value="sunset"${currentEnv === 'sunset' ? ' selected' : ''}>🌇 Fim de Tarde</option>
            <option value="night"${currentEnv === 'night' ? ' selected' : ''}>🌙 Noite (Vaga-lumes)</option>
            <option value="rain"${currentEnv === 'rain' ? ' selected' : ''}>🌧️ Chuva Pantaneira</option>
            <option value="none"${currentEnv === 'none' ? ' selected' : ''}>Sem efeito</option>
          </select>
        </label>
      </div>
      <p style="margin:6px 0 0;font-size:10.5px;color:var(--sub);line-height:1.35;">Atalhos: <kbd>4</kbd> pet, <kbd>5</kbd> clima, <kbd>J</kbd> dev, <kbd>L</kbd> bem-estar.</p>
    </div>
  `);
  document.getElementById('wardrobeHat')?.addEventListener('change', event => {
    const value = (event.target as HTMLSelectElement).value;
    try {
      localStorage.setItem('capy_hat', value);
      window.dispatchEvent(new StorageEvent('storage', { key: 'capy_hat', newValue: value }));
    } catch {}
  });
  document.getElementById('wardrobeClothes')?.addEventListener('change', event => {
    const value = (event.target as HTMLSelectElement).value;
    try {
      localStorage.setItem('capy_clothes', value);
      window.dispatchEvent(new StorageEvent('storage', { key: 'capy_clothes', newValue: value }));
    } catch {}
  });
  document.getElementById('wardrobeCostume')?.addEventListener('change', event => {
    const value = (event.target as HTMLSelectElement).value;
    try {
      localStorage.setItem('capy_costume', value);
      window.dispatchEvent(new StorageEvent('storage', { key: 'capy_costume', newValue: value }));
    } catch {}
  });
  document.getElementById('pantanalFriend')?.addEventListener('change', event => {
    const value = (event.target as HTMLSelectElement).value;
    try {
      localStorage.setItem('capy_friend', value);
      window.dispatchEvent(new StorageEvent('storage', { key: 'capy_friend', newValue: value }));
    } catch {}
  });
  document.getElementById('pantanalEnv')?.addEventListener('change', event => {
    const value = (event.target as HTMLSelectElement).value;
    try {
      localStorage.setItem('capy_env', value);
      window.dispatchEvent(new StorageEvent('storage', { key: 'capy_env', newValue: value }));
    } catch {}
  });
  controls.insertAdjacentHTML('afterend', '<section class="preferences" id="quotaPreferences"><h2>Alertas por conta</h2><p>Cada percentual avisa uma vez por janela. Se vários forem ultrapassados juntos, mostramos o maior.</p><div id="quotaRuleForms"></div></section>');
  document.getElementById('quotaPreferences')!.insertAdjacentHTML('beforeend','<p>Continuidade automática: sessões e chats Claude por assinatura. Alternativas API são usadas no chat e cobram por uso; a revisão exige confirmar a mudança de cobrança. A autenticação e o modelo API são conferidos pelo provedor ao enviar.</p>');
  document.getElementById('quotaRuleForms')!.addEventListener('change',event=>{
    const select=(event.target as Element).closest<HTMLSelectElement>('[data-destination-profile]');
    if(select)select.closest('.fallback-row')!.querySelector<HTMLInputElement>('[data-destination-model]')!.value='';
  });
  document.getElementById('quotaRuleForms')!.addEventListener('click', event => {
    const destination=(event.target as Element).closest('[data-add-destination]');
    if(destination){destination.closest('form')!.querySelector('.fallback-chain')!.insertAdjacentHTML('beforeend',fallbackMarkup(destinations(),'',profiles.length?'sonnet':''));return;}
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
      forms.innerHTML = rules.length ? rules.map((rule,index)=>ruleMarkup(rule,index,destinations())).join('') : '<p>As contas aparecem quando a integração confirmar sua identidade.</p>';
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
  const html=quotaNotifications(data);
  if (alerts.innerHTML!==html) alerts.innerHTML=html;
  alerts.hidden=!html;
}
