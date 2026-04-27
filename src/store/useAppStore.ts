import { type LolDeathPayload, Settings } from "@/lib/tauri";
import { create } from "zustand";

export type DeathRecord = LolDeathPayload & { timestamp: number };

type AppState = {
  settings: Settings;
  gameConnected: boolean;
  deaths: DeathRecord[];
  setSettings: (settings: Settings) => void;
  patchSettings: (settings: Partial<Settings>) => void;
  setGameConnected: (connected: boolean) => void;
  addDeath: (death: DeathRecord) => void;
  clearDeaths: () => void;
};

export const defaultSettings: Settings = {
  provider: "gemini",
  selected_model: "gemini-2.5-flash-lite",
  censorship_enabled: false,
  insult_preset: "brutal",
  volume: 0.8,
  speech_volume: 0.8,
  overlay_enabled: true,
  speech_enabled: false,
  saved_api_key_providers: [],
}

export const useAppStore = create<AppState>((set) => ({
  settings: defaultSettings,
  gameConnected: false,
  deaths: [],
  setSettings: (settings) => set({ settings }),
  patchSettings: (patch) =>
    set((state) => ({
      settings: {
        ...state.settings,
        ...patch
      }
    })),
  setGameConnected: (connected) => set({ gameConnected: connected }),
  addDeath: (death) =>
    set((state) => ({
      deaths: [...state.deaths, death].slice(-50),
    })),
  clearDeaths: () => set({ deaths: [] }),
}))
