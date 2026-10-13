import { check } from '@tauri-apps/plugin-updater';
import { native } from './bridge';

export async function checkAndApplyUpdates(
  onNotify?: (message: string, durationMs?: number) => void,
  manual = false
): Promise<boolean> {
  if (!native) {
    if (manual && onNotify) {
      onNotify('Atualização automática disponível apenas no app nativo! 🦫', 4000);
    }
    return false;
  }

  try {
    const update = await check();
    if (!update) {
      if (manual && onNotify) {
        onNotify('Capy já está na versão mais recente! ✨', 4000);
      }
      return false;
    }

    if (onNotify) {
      onNotify(`Nova versão v${update.version} encontrada! Baixando... 🚀`, 5000);
    }

    await update.downloadAndInstall();

    if (onNotify) {
      onNotify(`Atualização pronta! Reiniciando... 🦫✨`, 4000);
    }

    return true;
  } catch (err: any) {
    const message = err?.message || String(err);
    if (manual && onNotify) {
      onNotify('Não foi possível verificar atualizações no momento. 💧', 4000);
    }
    console.warn('[Updater] Check failed:', message);
    return false;
  }
}
