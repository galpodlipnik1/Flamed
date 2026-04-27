import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import type { DeathRecord } from '@/store/useAppStore';
import { ChevronDown, ChevronUp, Skull, Trash2 } from 'lucide-react';
import { useState } from 'react';

function formatGameTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${s.toString().padStart(2, '0')}`;
}

interface DeathHistoryProps {
  deaths: DeathRecord[];
  onClear: () => void;
}

export function DeathHistory({ deaths, onClear }: DeathHistoryProps) {
  const [open, setOpen] = useState(true);

  return (
    <section className="rounded-md border border-border bg-secondary/30 p-4">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Label className="text-xs font-semibold uppercase tracking-[0.14em] text-foreground">
            Death Log
          </Label>
          {deaths.length > 0 && (
            <Badge
              variant="secondary"
              className="rounded px-1.5 py-0 text-[10px] tabular-nums"
            >
              {deaths.length}
            </Badge>
          )}
        </div>

        <div className="flex items-center gap-1">
          {deaths.length > 0 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onClear}
              aria-label="Clear death log"
              className="h-7 px-2 text-muted-foreground hover:text-destructive"
            >
              <Trash2 className="size-3.5" aria-hidden="true" />
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setOpen((v) => !v)}
            aria-label={open ? 'Collapse death log' : 'Expand death log'}
            className="h-7 px-2 text-muted-foreground"
          >
            {open
              ? <ChevronUp className="size-3.5" aria-hidden="true" />
              : <ChevronDown className="size-3.5" aria-hidden="true" />}
          </Button>
        </div>
      </div>

      {open && (
        <div className="mt-3">
          {deaths.length === 0 ? (
            <p className="text-sm text-muted-foreground">No deaths this session. Stay alive!</p>
          ) : (
            <div className="flex max-h-72 flex-col gap-2 overflow-y-auto pr-0.5">
              {[...deaths].reverse().map((death) => (
                <div
                  key={death.timestamp}
                  className="rounded-md border border-border bg-background p-3"
                >
                  <p className="text-sm leading-snug text-foreground">{death.insult}</p>

                  <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
                    <span>
                      killed by{' '}
                      <span className="text-foreground">{death.killer}</span>
                    </span>

                    {death.deathStreak > 1 && (
                      <span className="flex items-center gap-1 text-primary">
                        <Skull className="size-3" aria-hidden="true" />
                        x{death.deathStreak} streak
                      </span>
                    )}

                    <span>{death.kda} KDA</span>
                    <span>@ {formatGameTime(death.gameTimeSeconds)}</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </section>
  );
}
