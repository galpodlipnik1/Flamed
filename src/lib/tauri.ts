import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Window } from "@tauri-apps/api/window";

export type AiProvider = "gemini" | "open_ai" | "anthropic";
export type ModelTier = "frontier" | "mid" | "budget";
export type InsultPreset = "warmup" | "salty" | "brutal" | "nuclear";

export type ProviderModel = {
  tier: ModelTier;
  label: string;
  model: string;
};

export type ProviderOption = {
  id: AiProvider;
  label: string;
  models: ProviderModel[];
};

export type Settings = {
  provider: AiProvider;
  selected_model: string;
  censorship_enabled: boolean;
  insult_preset: InsultPreset;
  volume: number;
  speech_volume: number;
  overlay_enabled: boolean;
  speech_enabled: boolean;
  saved_api_key_providers: AiProvider[];
}

export type LoadSettingsResult = {
  settings: Settings;
  warning?: string | null;
};

export type LolDeathPayload = {
  killer: string;
  deathStreak: number;
  kda: string;
  gameTimeSeconds: number;
  insult: string;
};

export type LolStatusPayload = {
  connected: boolean;
  message?: string;
};

export type LolGameEndPayload = {
  result: 'win' | 'lose';
  message: string;
  kda: string;
  gameTimeSeconds: number;
};

export function loadSettings() { 
  return invoke<LoadSettingsResult>("load_settings");
}

export function saveSettings(settings: Settings) {
  const { provider, selected_model, censorship_enabled, insult_preset, volume, speech_volume, overlay_enabled, speech_enabled } = settings;
  return invoke<Settings>("save_settings", {
    settings: { provider, selected_model, censorship_enabled, insult_preset, volume, speech_volume, overlay_enabled, speech_enabled },
  });
}

export function providerOptions() {
  return invoke<ProviderOption[]>("provider_options");
}

export function setProviderApiKey(provider: AiProvider, apiKey: string) {
  return invoke<Settings>("set_provider_api_key", { provider, apiKey });
}

export function clearProviderApiKey(provider: AiProvider) {
  return invoke<Settings>("clear_provider_api_key", { provider });
}

export function testSavedApiKey(provider: AiProvider) {
  return invoke<boolean>("test_saved_api_key", { provider });
}

export function generateInsult(
  killer: string,
  champion: string,
  deathStreak: number,
  kda: string,
  gameTimeSeconds: number
) {
  return invoke<string>("generate_insult", {
    killer,
    champion,
    deathStreak,
    kda,
    gameTimeSeconds,
  });
}

export function playDeathSound(volume: number) {
  return invoke<void>("play_death_sound", { volume });
}

export function onLolDeath(handler: (payload: LolDeathPayload) => void): Promise<UnlistenFn> { 
  return listen<LolDeathPayload>("lol-death", (event) => handler(event.payload));
}

export function onLolStatus(handler: (payload: LolStatusPayload) => void): Promise<UnlistenFn> {
  return listen<LolStatusPayload>("lol-status", (event) => handler(event.payload));
}

export function onLolGameEnd(handler: (payload: LolGameEndPayload) => void): Promise<UnlistenFn> {
  return listen<LolGameEndPayload>("lol-game-end", (event) => handler(event.payload));
}

export async function showOverlayWindow() { 
  const overlay = await Window.getByLabel("overlay");
  await overlay?.show();
}

export async function hideOverlayWindow() { 
  const overlay = await Window.getByLabel("overlay");
  await overlay?.hide();
}
