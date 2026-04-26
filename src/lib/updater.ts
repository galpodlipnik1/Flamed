import { check } from '@tauri-apps/plugin-updater';

export async function checkForUpdates(): Promise<void> {
  let update;

  try {
    update = await check();
  } catch (error) {
    console.warn('Update check failed', error);
    return;
  }

  if (!update) return;

  const confirmed = window.confirm(`Flamed ${update.version} is available.\n\n${update.body ?? 'Bug fixes and improvements.'}\n\nInstall now? The app will restart.`);

  if (!confirmed) return;

  await update.downloadAndInstall();
}
