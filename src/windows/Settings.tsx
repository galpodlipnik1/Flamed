import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Card,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import {
    clearProviderApiKey,
    loadSettings,
    onLolDeath,
    onLolStatus,
    providerOptions,
    saveSettings,
    setProviderApiKey,
    testSavedApiKey,
    type AiProvider,
    type InsultPreset,
    type ProviderOption,
} from '@/lib/tauri';
import { DeathHistory } from '@/components/DeathHistory';
import { UpdateDialog } from '@/components/UpdateDialog';
import { checkForUpdates, type Update } from '@/lib/updater';
import { useAppStore } from '@/store/useAppStore';
import { CheckCircle2, Loader2, Save, TestTube2, Volume2 } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';

const INSULT_PRESETS: { value: InsultPreset; label: string }[] = [
  { value: 'warmup', label: '1 - Warmup' },
  { value: 'salty', label: '2 - Salty' },
  { value: 'brutal', label: '3 - Brutal' },
  { value: 'nuclear', label: '4 - Nuclear' },
];

export function Settings() {
  const { settings, setSettings, patchSettings, gameConnected, setGameConnected, deaths, addDeath, clearDeaths } = useAppStore();
  const [pendingUpdate, setPendingUpdate] = useState<Update | null>(null);
  const [loading, setLoading] = useState(true);
  const [testingSavedKey, setTestingSavedKey] = useState(false);
  const [saving, setSaving] = useState(false);
  const [savingApiKey, setSavingApiKey] = useState(false);
  const [clearingApiKey, setClearingApiKey] = useState(false);
  const [loadedOnce, setLoadedOnce] = useState(false);
  const [apiKeyDraft, setApiKeyDraft] = useState('');
  const [providers, setProviders] = useState<ProviderOption[]>([]);

  const volumePercent = useMemo(() => Math.round(settings.volume * 100), [settings.volume]);
  const speechVolumePercent = useMemo(() => Math.round(settings.speech_volume * 100), [settings.speech_volume]);
  const activeProvider = useMemo(
    () => providers.find((provider) => provider.id === settings.provider),
    [providers, settings.provider],
  );
  const activeProviderHasSavedKey = settings.saved_api_key_providers.includes(settings.provider);
  const speechAvailable =
    settings.provider === 'gemini' && settings.saved_api_key_providers.includes('gemini');

  useEffect(() => {
    let cancelled = false;
    let unlistenStatus: (() => void) | undefined;

    Promise.all([loadSettings(), providerOptions()])
      .then(([loaded, loadedProviders]) => {
        if (!cancelled) {
          setProviders(loadedProviders);
          setSettings(loaded.settings);
          setLoadedOnce(true);
          if (loaded.warning) {
            toast.warning(loaded.warning);
          }
        }
      })
      .catch((error) => toast.error(String(error)))
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    onLolStatus((payload) => {
      setGameConnected(payload.connected);
      if (payload.connected && payload.message) {
        toast.error(payload.message);
      }
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      unlistenStatus = unlisten;
    });

    return () => {
      cancelled = true;
      unlistenStatus?.();
    };
  }, [setGameConnected, setSettings]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    onLolDeath((payload) => {
      addDeath({ ...payload, timestamp: Date.now() });
    }).then((fn) => {
      unlisten = fn;
    });

    return () => unlisten?.();
  }, [addDeath]);

  useEffect(() => {
    if (loadedOnce && !speechAvailable && settings.speech_enabled) {
      patchSettings({ speech_enabled: false });
    }
  }, [loadedOnce, patchSettings, settings.speech_enabled, speechAvailable]);

  useEffect(() => {
    if (!loadedOnce) return;

    const handle = window.setTimeout(() => {
      saveSettings(settings).catch((error) => toast.error(String(error)));
    }, 350);

    return () => window.clearTimeout(handle);
  }, [loadedOnce, settings]);

  useEffect(() => {
    checkForUpdates()
      .then((update) => { if (update) setPendingUpdate(update); })
      .catch(() => {});
  }, []);

  async function handleSave() {
    setSaving(true);
    try {
      const saved = await saveSettings(settings);
      setSettings(saved);
      toast.success('Settings saved');
    } catch (error) {
      toast.error(String(error));
    } finally {
      setSaving(false);
    }
  }

  async function handleSaveApiKey() {
    setSavingApiKey(true);
    try {
      const saved = await setProviderApiKey(settings.provider, apiKeyDraft);
      setSettings(saved);
      setApiKeyDraft('');
      toast.success(`${activeProvider?.label ?? 'Provider'} API key saved securely`);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setSavingApiKey(false);
    }
  }

  async function handleClearApiKey() {
    setClearingApiKey(true);
    try {
      const saved = await clearProviderApiKey(settings.provider);
      setSettings({
        ...saved,
        speech_enabled: settings.provider === 'gemini' ? false : saved.speech_enabled,
      });
      setApiKeyDraft('');
      toast.success(`${activeProvider?.label ?? 'Provider'} API key cleared`);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setClearingApiKey(false);
    }
  }

  async function handleTestSavedApiKey() {
    setTestingSavedKey(true);
    try {
      const ok = await testSavedApiKey(settings.provider);
      if (ok) {
        toast.success(`Saved ${activeProvider?.label ?? 'provider'} API key works`);
      } else {
        toast.error('Saved API key is invalid');
      }
    } catch (error) {
      toast.error(String(error));
    } finally {
      setTestingSavedKey(false);
    }
  }

  function handleProviderChange(provider: AiProvider) {
    const nextProvider = providers.find((option) => option.id === provider);
    const nextModel =
      nextProvider?.models[nextProvider.models.length - 1]?.model ?? settings.selected_model;
    patchSettings({
      provider,
      selected_model: nextModel,
      speech_enabled: provider === 'gemini' ? settings.speech_enabled : false,
    });
    setApiKeyDraft('');
  }

  return (
    <main className="min-h-screen bg-background text-foreground">
      {pendingUpdate && (
        <UpdateDialog update={pendingUpdate} onDismiss={() => setPendingUpdate(null)} />
      )}
      <div className="mx-auto flex min-h-screen w-full max-w-140 flex-col p-4">
        <Card className="flex min-h-[calc(100vh-2rem)] flex-col rounded-lg border border-border bg-card shadow-[0_20px_60px_rgba(0,0,0,0.35)]">
          <CardHeader className="gap-0 border-b border-border px-6 py-5">
            <div className="flex items-start justify-between gap-4">
              <div className="flex min-w-0 items-center gap-4">
                <div className="flex size-12 shrink-0 items-center justify-center rounded-lg bg-primary text-primary-foreground shadow-[0_10px_30px_rgba(255,40,40,0.28)]">
                  <img src="/flamed.svg" alt="" aria-hidden="true" className="size-12 rounded-lg" />
                </div>

                <div className="min-w-0">
                  <CardTitle className="font-display text-3xl uppercase leading-none tracking-normal">
                    Flamed
                  </CardTitle>
                  <CardDescription className="mt-2 text-sm leading-relaxed text-muted-foreground">
                    Get F**ed by AI when you die.
                  </CardDescription>
                </div>
              </div>

              <Badge
                variant={gameConnected ? 'default' : 'secondary'}
                className="mt-1 shrink-0 rounded-md px-3 py-1 text-[11px] uppercase tracking-[0.12em]"
              >
                {gameConnected ? 'Game detected' : 'Waiting for game...'}
              </Badge>
            </div>
          </CardHeader>

          <CardContent className="flex flex-1 flex-col gap-6 overflow-y-auto px-6 py-6">
            {loading ? (
              <div className="flex flex-1 items-center justify-center text-muted-foreground">
                <Loader2 className="mr-2 animate-spin" aria-hidden="true" />
                Loading settings
              </div>
            ) : (
              <>
                <section className="flex flex-col gap-3">
                  <div className="grid gap-3 sm:grid-cols-2">
                    <div className="flex flex-col gap-2">
                      <Label
                        htmlFor="provider"
                        className="text-xs font-semibold uppercase tracking-[0.14em] text-foreground"
                      >
                        Provider
                      </Label>
                      <select
                        id="provider"
                        value={settings.provider}
                        onChange={(event) => handleProviderChange(event.target.value as AiProvider)}
                        className="h-12 rounded-md border border-border bg-background px-3 text-base text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
                      >
                        {providers.map((provider) => (
                          <option key={provider.id} value={provider.id}>
                            {provider.label}
                          </option>
                        ))}
                      </select>
                    </div>

                    <div className="flex flex-col gap-2">
                      <Label
                        htmlFor="selected_model"
                        className="text-xs font-semibold uppercase tracking-[0.14em] text-foreground"
                      >
                        Model
                      </Label>
                      <select
                        id="selected_model"
                        value={settings.selected_model}
                        onChange={(event) => patchSettings({ selected_model: event.target.value })}
                        className="h-12 rounded-md border border-border bg-background px-3 text-base text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
                      >
                        {activeProvider?.models.map((model) => (
                          <option key={model.model} value={model.model}>
                            {model.label}
                          </option>
                        ))}
                      </select>
                    </div>
                  </div>

                  <div className="flex items-center justify-between gap-3">
                    <Label
                      htmlFor="gemini_api_key"
                      className="text-xs font-semibold uppercase tracking-[0.14em] text-foreground"
                    >
                      {activeProvider?.label ?? 'Provider'} API Key
                    </Label>

                    <Badge variant={activeProviderHasSavedKey ? 'default' : 'secondary'}>
                      {activeProviderHasSavedKey ? 'Saved securely' : 'Not saved'}
                    </Badge>
                  </div>

                  <Input
                    id="gemini_api_key"
                    type="password"
                    value={apiKeyDraft}
                    placeholder={
                      activeProviderHasSavedKey
                        ? 'Paste a new key to replace the saved one'
                        : `Paste your ${activeProvider?.label ?? 'provider'} API key here`
                    }
                    onChange={(event) => setApiKeyDraft(event.target.value)}
                    className="h-12 rounded-md border-border bg-background px-4 text-base"
                  />

                  <div className="flex flex-wrap gap-2">
                    <Button
                      type="button"
                      size="sm"
                      onClick={handleSaveApiKey}
                      disabled={savingApiKey || apiKeyDraft.trim().length === 0}
                    >
                      {savingApiKey ? (
                        <Loader2
                          className="animate-spin"
                          data-icon="inline-start"
                          aria-hidden="true"
                        />
                      ) : (
                        <Save data-icon="inline-start" aria-hidden="true" />
                      )}
                      Save Key
                    </Button>

                    <Button
                      type="button"
                      variant="secondary"
                      size="sm"
                      onClick={handleTestSavedApiKey}
                      disabled={testingSavedKey || !activeProviderHasSavedKey}
                    >
                      {testingSavedKey ? (
                        <Loader2
                          className="animate-spin"
                          data-icon="inline-start"
                          aria-hidden="true"
                        />
                      ) : (
                        <TestTube2 data-icon="inline-start" aria-hidden="true" />
                      )}
                      Test Saved
                    </Button>

                    <Button
                      type="button"
                      variant="secondary"
                      size="sm"
                      onClick={handleClearApiKey}
                      disabled={clearingApiKey || !activeProviderHasSavedKey}
                    >
                      {clearingApiKey ? (
                        <Loader2
                          className="animate-spin"
                          data-icon="inline-start"
                          aria-hidden="true"
                        />
                      ) : null}
                      Clear
                    </Button>
                  </div>
                </section>

                <section className="rounded-md border border-border bg-secondary/30 p-4">
                  <div className="flex flex-col gap-2">
                    <Label
                      htmlFor="insult_preset"
                      className="text-xs font-semibold uppercase tracking-[0.14em] text-foreground"
                    >
                      Insult Level
                    </Label>
                    <select
                      id="insult_preset"
                      value={settings.insult_preset}
                      onChange={(event) =>
                        patchSettings({ insult_preset: event.target.value as InsultPreset })
                      }
                      className="h-12 rounded-md border border-border bg-background px-3 text-base text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                      {INSULT_PRESETS.map((preset) => (
                        <option key={preset.value} value={preset.value}>
                          {preset.label}
                        </option>
                      ))}
                    </select>
                  </div>
                </section>

                <section className="rounded-md border border-border bg-secondary/30 p-4">
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex flex-col gap-1">
                      <Label
                        htmlFor="speech_enabled"
                        className="text-xs font-semibold uppercase tracking-[0.14em] text-foreground"
                      >
                        Speech Synthesis
                      </Label>
                      <p className="text-sm text-muted-foreground">
                        {speechAvailable
                          ? 'Speaks roasts using Gemini TTS.'
                          : 'Requires Gemini as the selected provider with a saved key.'}
                      </p>
                    </div>

                    <Switch
                      id="speech_enabled"
                      checked={speechAvailable && settings.speech_enabled}
                      disabled={!speechAvailable}
                      onCheckedChange={(checked) => {
                        if (speechAvailable) {
                          patchSettings({ speech_enabled: checked });
                        }
                      }}
                    />
                  </div>
                </section>

                <section className="rounded-md border border-border bg-secondary/30 p-4">
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex flex-col gap-1">
                      <Label
                        htmlFor="censorship_enabled"
                        className="text-xs font-semibold uppercase tracking-[0.14em] text-foreground"
                      >
                        Censorship
                      </Label>
                      <p className="text-sm text-muted-foreground">
                        Asks the model to return masked profanity.
                      </p>
                    </div>

                    <Switch
                      id="censorship_enabled"
                      checked={settings.censorship_enabled}
                      onCheckedChange={(checked) => patchSettings({ censorship_enabled: checked })}
                    />
                  </div>
                </section>

                <section className="rounded-md border border-border bg-secondary/30 p-4">
                  <div className="mb-4 flex items-center justify-between gap-4">
                    <div className="flex items-center gap-2">
                      <Volume2 className="size-4 text-primary" aria-hidden="true" />
                      <Label
                        htmlFor="volume"
                        className="text-xs font-semibold uppercase tracking-[0.14em] text-foreground"
                      >
                        Death Sound Volume
                      </Label>
                    </div>

                    <span className="text-sm text-muted-foreground">{volumePercent}%</span>
                  </div>

                  <Slider
                    id="volume"
                    value={volumePercent}
                    max={100}
                    step={1}
                    onValueChange={(value) => patchSettings({ volume: value / 100 })}
                  />

                  {speechAvailable && (
                    <>
                      <div className="mb-4 mt-5 flex items-center justify-between gap-4">
                        <div className="flex items-center gap-2">
                          <Volume2 className="size-4 text-primary" aria-hidden="true" />
                          <Label
                            htmlFor="speech_volume"
                            className="text-xs font-semibold uppercase tracking-[0.14em] text-foreground"
                          >
                            Speech Volume
                          </Label>
                        </div>

                        <span className="text-sm text-muted-foreground">{speechVolumePercent}%</span>
                      </div>

                      <Slider
                        id="speech_volume"
                        value={speechVolumePercent}
                        max={100}
                        step={1}
                        onValueChange={(value) => patchSettings({ speech_volume: value / 100 })}
                      />
                    </>
                  )}
                </section>

                <section className="rounded-md border border-border bg-secondary/30 p-4">
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex flex-col gap-1">
                      <Label
                        htmlFor="overlay_enabled"
                        className="text-xs font-semibold uppercase tracking-[0.14em] text-foreground"
                      >
                        Overlay Enabled
                      </Label>
                      <p className="text-sm text-muted-foreground">
                        Shows the roast banner and plays audio when you die.
                      </p>
                    </div>

                    <Switch
                      id="overlay_enabled"
                      checked={settings.overlay_enabled}
                      onCheckedChange={(checked) => patchSettings({ overlay_enabled: checked })}
                    />
                  </div>
                </section>

                <DeathHistory deaths={deaths} onClear={clearDeaths} />

                <section className="rounded-md border border-primary/20 bg-primary/5 p-4">
                  <div className="flex items-start gap-3">
                    <CheckCircle2 className="mt-0.5 size-4 text-primary" aria-hidden="true" />
                    <p className="text-sm leading-relaxed text-muted-foreground">
                      Settings auto-save after edits. Use{' '}
                      <span className="text-foreground">Save</span> if you want an explicit write
                      immediately.
                    </p>
                  </div>
                </section>
              </>
            )}
          </CardContent>

          <Separator />

          <CardFooter className="items-center justify-between px-6 py-5">
            <span className="text-xs text-muted-foreground">v1.0.3</span>

            <Button onClick={handleSave} disabled={saving || loading}>
              {saving ? (
                <Loader2 className="animate-spin" data-icon="inline-start" aria-hidden="true" />
              ) : (
                <Save data-icon="inline-start" aria-hidden="true" />
              )}
              Save
            </Button>
          </CardFooter>
        </Card>
      </div>
    </main>
  );
}
