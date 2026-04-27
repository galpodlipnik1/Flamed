import { check, type Update } from '@tauri-apps/plugin-updater';

export type { Update };

export async function checkForUpdates(): Promise<Update | null> {
  try {
    return await check();
  } catch (error) {
    console.warn('Update check failed', error);
    return null;
  }
}
