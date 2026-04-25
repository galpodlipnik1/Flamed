# Flamed

**Get roasted by AI every time you die in League of Legends.**

Flamed is a Windows desktop overlay that watches your game, detects when you die, and fires an AI-generated insult at you in real time — text on screen and optionally spoken out loud.

---

## Features

- **Real-time death detection** — hooks into the League of Legends live client API, no game files touched
- **AI-generated roasts** — uses your choice of Gemini, OpenAI, or Anthropic to write a unique insult every death
- **Champion-aware** — the AI knows what champion you're playing and uses it
- **Death streak escalation** — the roasts get progressively more brutal the more you feed
- **Text-to-speech** — Gemini TTS reads the insult out loud in a furious, disappointed voice (Gemini only)
- **Four insult levels** — Warmup, Salty, Brutal, and Nuclear
- **Censorship toggle** — mask profanity or let it rip
- **Separate volume controls** — death sound and speech volume are independent
- **Always-on-top overlay** — transparent, click-through, sits at the bottom of your screen
- **System tray app** — runs quietly in the background

---

## Installation

Download the latest installer from the [Releases](../../releases/latest) page and run it.

> Windows may show a SmartScreen warning since the exe is unsigned. Click **More info → Run anyway** to proceed. This is safe — the source code is fully open.

---

## Setup

Flamed needs an API key from at least one AI provider to generate insults.

### Getting an API key

| Provider | Free tier | Link |
|---|---|---|
| **Gemini** (recommended) | Yes — generous free quota | [aistudio.google.com](https://aistudio.google.com/apikey) |
| **OpenAI** | No — pay per use | [platform.openai.com](https://platform.openai.com/api-keys) |
| **Anthropic** | No — pay per use | [console.anthropic.com](https://console.anthropic.com/) |

Gemini is recommended because it's free and also powers the text-to-speech feature.

### Configuring the app

1. Launch Flamed — it appears in the system tray
2. Click the tray icon to open Settings
3. Select your provider and paste your API key
4. Click **Save Key**, then **Test Saved** to verify it works
5. Pick your insult level and launch a game

The overlay will appear at the bottom of your screen the next time you die.

---

## Insult Levels

| Level | Description |
|---|---|
| **1 — Warmup** | Clever jab, real bite, no heavy cruelty |
| **2 — Salty** | Mean, annoyed, mocking |
| **3 — Brutal** | Harsh humiliation, contempt, profanity welcome |
| **4 — Nuclear** | Soul-destroying obliteration. Genuinely upsetting. |

Nuclear with a 5+ death streak triggers **Feeding Frenzy** mode — the AI is instructed to treat you as having hit absolute rock bottom with zero restraint.

---

## Text-to-Speech

When using Gemini as your provider, you can enable **Speech Synthesis** in Settings. The insult will be spoken out loud by Gemini TTS in a furious, barely-contained-rage voice every time you die.

Speech volume is controlled by its own slider, separate from the death sound effect.

---

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/)
- [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (Windows, required by Tauri)

### Commands

```bash
# Install dependencies
bun install

# Run in development mode
bun run tauri dev

# Build a release installer
bun run tauri build
```

The built installer will be at `src-tauri/target/release/bundle/nsis/`.

---

## How It Works

1. A background loop polls the [League of Legends Live Client API](https://developer.riotgames.com/docs/lol#league-client-api_live-client-data-api) at `https://127.0.0.1:2999` — this API is available whenever a game is running
2. When a `ChampionKill` event is detected with you as the victim, it fetches your KDA, champion, game time, and killer
3. These are sent to the AI provider with a structured prompt tuned to your selected insult level
4. The roast is displayed in the always-on-top overlay and optionally spoken via Gemini TTS
5. The overlay auto-dismisses after a read-time calculated from the insult length

---

## Contributing

Issues and pull requests are welcome. The codebase is a [Tauri v2](https://v2.tauri.app/) app — Rust backend in `src-tauri/src/`, React/TypeScript frontend in `src/`.

Key files:

| File | Purpose |
|---|---|
| `src-tauri/src/lol.rs` | Game polling and death detection |
| `src-tauri/src/ai.rs` | Prompt building and AI calls |
| `src-tauri/src/speech.rs` | Gemini TTS |
| `src-tauri/src/settings.rs` | Settings persistence |
| `src/windows/Overlay.tsx` | Death overlay UI |
| `src/windows/Settings.tsx` | Settings panel UI |

---

## License

MIT
