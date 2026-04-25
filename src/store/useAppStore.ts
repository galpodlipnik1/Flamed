import { Settings } from "@/lib/tauri";
import { create } from "zustand";

type AppState = {
  settings: Settings;
  gameConnected: boolean;
  setSettings: (settings: Settings) => void;
  patchSettings: (settings: Partial<Settings>) => void;
  setGameConnected: (connected: boolean) => void;
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
  setSettings: (settings) => set({ settings }),
  patchSettings: (patch) =>
    set((state) => ({
      settings: {
        ...state.settings,
        ...patch
      }
    })),
  setGameConnected: (connected) => set({ gameConnected: connected }),
}))
