import { Button } from '@/components/ui/button';
import type { Update } from '@/lib/updater';
import { ArrowDownToLine, X } from 'lucide-react';
import { useState } from 'react';

interface UpdateDialogProps {
  update: Update;
  onDismiss: () => void;
}

export function UpdateDialog({ update, onDismiss }: UpdateDialogProps) {
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);

  async function handleInstall() {
    setInstalling(true);
    let downloaded = 0;
    let total = 0;

    await update.downloadAndInstall((event) => {
      if (event.event === 'Started') {
        total = (event.data as { contentLength?: number }).contentLength ?? 0;
      } else if (event.event === 'Progress') {
        downloaded += (event.data as { chunkLength: number }).chunkLength;
        if (total > 0) {
          setProgress(Math.round((downloaded / total) * 100));
        }
      } else if (event.event === 'Finished') {
        setProgress(100);
      }
    });
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="mx-4 w-full max-w-sm animate-[dialog-in_200ms_ease-out_both] rounded-lg border border-border bg-card shadow-[0_20px_60px_rgba(0,0,0,0.6)]">
        <div className="flex items-start justify-between gap-4 border-b border-border px-5 py-4">
          <div className="flex items-center gap-3">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-md bg-primary/15 text-primary shadow-[0_0_16px_rgba(255,40,40,0.18)]">
              <ArrowDownToLine className="size-5" aria-hidden="true" />
            </div>
            <div>
              <h2 className="font-display text-lg uppercase leading-none tracking-wide text-foreground">
                Update Available
              </h2>
              <p className="mt-1 text-xs text-muted-foreground">Flamed {update.version}</p>
            </div>
          </div>

          {!installing && (
            <button
              onClick={onDismiss}
              aria-label="Dismiss"
              className="mt-0.5 rounded text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              <X className="size-4" />
            </button>
          )}
        </div>

        <div className="px-5 py-4">
          <p className="line-clamp-6 whitespace-pre-line text-sm leading-relaxed text-muted-foreground">
            {update.body?.trim() || 'Bug fixes and improvements.'}
          </p>

          {installing && (
            <div className="mt-4">
              <div className="mb-1.5 flex items-center justify-between">
                <span className="text-xs text-muted-foreground">
                  {progress === null
                    ? 'Preparing…'
                    : progress < 100
                      ? 'Downloading…'
                      : 'Installing…'}
                </span>
                {progress !== null && (
                  <span className="text-xs tabular-nums text-muted-foreground">{progress}%</span>
                )}
              </div>

              {progress === null ? (
                <div className="h-1.5 w-full animate-pulse rounded-full bg-primary/40" />
              ) : (
                <div className="h-1.5 w-full overflow-hidden rounded-full bg-secondary">
                  <div
                    className="h-full rounded-full bg-primary transition-all duration-300"
                    style={{ width: `${progress}%` }}
                  />
                </div>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-border px-5 py-4">
          {!installing && (
            <Button variant="secondary" size="sm" onClick={onDismiss}>
              Later
            </Button>
          )}
          <Button size="sm" onClick={handleInstall} disabled={installing}>
            {installing ? 'Installing…' : 'Install & Restart'}
          </Button>
        </div>
      </div>
    </div>
  );
}
