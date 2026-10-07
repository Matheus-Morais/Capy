import { listProfiles, addProfile, profileIdentity, loginProfile, connectClaudeQuotas, native, showError, type AccountProfile, type AccountIdentity } from './bridge';
import { profileMarkup } from './presentation';

export async function initializeAccounts(): Promise<void> {
  if (document.body.dataset.surface !== 'panel') return;
  const host = document.createElement('section');
  host.className = 'accounts'; host.id='accounts';
  document.getElementById('sessions')!.after(host);
  if (!native) { host.innerHTML='<h2>Contas Claude</h2><p>Cadastro e login oficial disponíveis no aplicativo desktop.</p>'; return; }
  host.innerHTML='<h2>Contas Claude</h2><p>Use uma configuração existente ou faça login em uma pasta separada. A forma de cobrança será confirmada pelo CLI.</p><div id="accountList"></div><details><summary>Adicionar conta</summary><form id="addAccount"><label>Nome da conta<input name="label" maxlength="80" required placeholder="Pessoal, trabalho…"></label><label>Configuração<select name="mode"><option value="new">Nova pasta isolada</option><option value="existing">Pasta de configuração existente</option></select></label><label id="existingConfig" hidden>Pasta de configuração<input name="existing" placeholder="C:\Users\…\.claude-trabalho"></label><label>Login desejado<select name="billing"><option value="subscription">Assinatura Claude</option><option value="api">Anthropic Console · cobrança por uso</option></select></label><button class="primary" type="submit">Cadastrar conta</button><p id="addAccountNotice" role="status"></p></form></details>';
  let profiles: AccountProfile[] = [];
  const identities = new Map<string, AccountIdentity>();
  const refresh = async () => {
    profiles = await listProfiles();
    document.getElementById('accountList')!.innerHTML = profiles.map(p=>profileMarkup(p,identities.get(p.id))).join('');
    window.dispatchEvent(new CustomEvent('capy-profiles', {detail:profiles}));
  };
  host.addEventListener('click',event=> {
    const button=(event.target as Element).closest<HTMLButtonElement>('[data-account-action]');
    if (!button || button.disabled) return;
    const row=button.closest<HTMLElement>('[data-profile]')!;
    const id=row.dataset.profile!;
    const notice=row.querySelector<HTMLElement>('[data-account-notice]')!;
    button.disabled=true;
    if (button.dataset.accountAction==='login') {
      void loginProfile(id).then(()=>{notice.textContent='Login oficial aberto no terminal. Ao terminar, clique em Verificar conta.';}).catch(showError).finally(()=>button.disabled=false);
    } else if (button.dataset.accountAction?.startsWith('quotas-')) {
      const enabled=button.dataset.accountAction==='quotas-on';
      void connectClaudeQuotas(id,enabled).then(()=>{notice.textContent=enabled?'Fonte de quotas conectada, preservando a statusline existente. Abra uma nova sessão e aguarde sua primeira resposta.':'Fonte de quotas desconectada; configuração original restaurada.';}).catch(showError).finally(()=>button.disabled=false);
    } else {
      notice.textContent='Consultando o CLI oficial…';
      void profileIdentity(id).then(identity=>{identities.set(id,identity);return refresh();}).catch(error=>{notice.textContent='Não foi possível confirmar a conta. Confira o CLI e tente novamente.';showError(error);}).finally(()=>button.disabled=false);
    }
  });
  const form=document.getElementById('addAccount') as HTMLFormElement;
  form.addEventListener('change',()=> {
    const existing=(form.elements.namedItem('mode') as HTMLSelectElement).value==='existing';
    document.getElementById('existingConfig')!.hidden=!existing;
    (form.elements.namedItem('existing') as HTMLInputElement).required=existing;
  });
  form.addEventListener('submit',event=>{
    event.preventDefault(); if (!form.reportValidity()) return;
    const values=new FormData(form);
    const button=form.querySelector<HTMLButtonElement>('[type="submit"]')!;
    const notice=document.getElementById('addAccountNotice')!;
    const existing=values.get('mode')==='existing';
    button.disabled=true;
    void addProfile(String(values.get('label')).trim(),existing?String(values.get('existing')).trim():null,String(values.get('billing'))).then(async profile=>{
      await refresh();
      if (!existing) {await loginProfile(profile.id);notice.textContent='Conta cadastrada. Termine o login oficial no terminal e verifique o vínculo.';}
      else notice.textContent='Configuração cadastrada. Verifique a conta para confirmar identidade e cobrança.';
      form.reset(); document.getElementById('existingConfig')!.hidden=true;
    }).catch(showError).finally(()=>button.disabled=false);
  });
  try{await refresh();}catch(error){
    document.getElementById('accountList')!.textContent=String(error);
    form.querySelectorAll<HTMLInputElement|HTMLButtonElement|HTMLSelectElement>('input,button,select').forEach(control=>control.disabled=true);
    showError(error);
  }
}
