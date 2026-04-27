import { Button } from '@/components/ui/button';
import {
    hideOverlayWindow,
    onLolDeath,
    onLolGameEnd,
    showOverlayWindow,
    type LolDeathPayload,
    type LolGameEndPayload,
} from '@/lib/tauri';
import { Skull, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

function getOverlayReadTime(insult: string) {
  const baseMs = 5200;
  const perCharMs = 42;
  const minMs = 6500;
  const maxMs = 14000;

  return Math.max(minMs, Math.min(maxMs, baseMs + insult.length * perCharMs));
}

export function Overlay() {
  const [death, setDeath] = useState<LolDeathPayload | null>(null);
  const [hiding, setHiding] = useState(false);
  const hideTimer = useRef<number | null>(null);
  const fadeTimer = useRef<number | null>(null);

  const [gameEnd, setGameEnd] = useState<LolGameEndPayload | null>(null);
  const [gameEndHiding, setGameEndHiding] = useState(false);
  const gameEndHideTimer = useRef<number | null>(null);
  const gameEndFadeTimer = useRef<number | null>(null);

  useEffect(() => {
    hideOverlayWindow().catch(() => {
      // Ignore
    });
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    onLolDeath(async (payload) => {
      if (hideTimer.current) window.clearTimeout(hideTimer.current);
      if (fadeTimer.current) window.clearTimeout(fadeTimer.current);

      setDeath(payload);
      setHiding(false);
      await showOverlayWindow();

      hideTimer.current = window.setTimeout(() => {
        setHiding(true);
        fadeTimer.current = window.setTimeout(async () => {
          await hideOverlayWindow();
          setDeath(null);
          setHiding(false);
        }, 280);
      }, getOverlayReadTime(payload.insult));
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
      if (hideTimer.current) window.clearTimeout(hideTimer.current);
      if (fadeTimer.current) window.clearTimeout(fadeTimer.current);
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    onLolGameEnd(async (payload) => {
      if (hideTimer.current) window.clearTimeout(hideTimer.current);
      if (fadeTimer.current) window.clearTimeout(fadeTimer.current);
      if (gameEndHideTimer.current) window.clearTimeout(gameEndHideTimer.current);
      if (gameEndFadeTimer.current) window.clearTimeout(gameEndFadeTimer.current);

      setDeath(null);
      setHiding(false);
      setGameEnd(payload);
      setGameEndHiding(false);
      await showOverlayWindow();

      gameEndHideTimer.current = window.setTimeout(() => {
        setGameEndHiding(true);
        gameEndFadeTimer.current = window.setTimeout(async () => {
          await hideOverlayWindow();
          setGameEnd(null);
          setGameEndHiding(false);
        }, 280);
      }, 10000);
    }).then((fn) => { unlisten = fn; });

    return () => {
      unlisten?.();
      if (gameEndHideTimer.current) window.clearTimeout(gameEndHideTimer.current);
      if (gameEndFadeTimer.current) window.clearTimeout(gameEndFadeTimer.current);
    };
  }, []);

  async function closeOverlay() {
    if (hideTimer.current) window.clearTimeout(hideTimer.current);
    if (fadeTimer.current) window.clearTimeout(fadeTimer.current);
    setDeath(null);
    setHiding(false);
    await hideOverlayWindow();
  }

  async function closeGameEnd() {
    if (gameEndHideTimer.current) window.clearTimeout(gameEndHideTimer.current);
    if (gameEndFadeTimer.current) window.clearTimeout(gameEndFadeTimer.current);
    setGameEnd(null);
    setGameEndHiding(false);
    await hideOverlayWindow();
  }

  if (gameEnd) {
    const isWin = gameEnd.result === 'win';
    const accent = isWin ? '#f59e0b' : '#ef4444';

    return (
      <main className="relative flex h-screen w-screen items-center justify-center overflow-hidden bg-black/70 px-8 text-white">
        <div className="absolute inset-x-0 top-0 h-px" style={{ backgroundColor: accent }} />
        <div className="absolute inset-x-0 bottom-0 h-px" style={{ backgroundColor: accent }} />

        <Button
          aria-label="Hide overlay"
          variant="ghost"
          size="icon"
          className="absolute right-3 top-3 z-10 size-11 rounded-md bg-black/50 text-white hover:bg-white/10"
          onClick={closeGameEnd}
        >
          <X aria-hidden="true" />
        </Button>

        <section
          className={[
            'flex w-full max-w-280 flex-col items-center justify-center gap-3 text-center',
            gameEndHiding ? 'animate-insult-out' : 'animate-insult-in',
          ].join(' ')}
        >
          <span
            className="font-display text-5xl uppercase leading-none tracking-wide"
            style={{ color: accent }}
          >
            {isWin ? 'Victory' : 'Defeat'}
          </span>
          <p className="font-display text-[2.2rem] font-bold uppercase leading-[0.95] text-white wrap-break-word text-pretty drop-shadow-[0_2px_18px_rgba(0,0,0,0.6)]">
            {gameEnd.message}
          </p>
        </section>
      </main>
    );
  }

  if (!death) {
    return <div className="h-screen w-screen bg-transparent" />;
  }

  return (
    <main className="relative flex h-screen w-screen items-center justify-center overflow-hidden bg-black/70 px-8 text-white">
      <div className="absolute inset-x-0 top-0 h-px bg-primary" />
      <div className="absolute inset-x-0 bottom-0 h-px bg-primary" />

      <Button
        aria-label="Hide overlay"
        variant="ghost"
        size="icon"
        className="absolute right-3 top-3 z-10 size-11 rounded-md bg-black/50 text-white hover:bg-primary hover:text-primary-foreground"
        onClick={closeOverlay}
      >
        <X aria-hidden="true" />
      </Button>

      <section
        className={[
          'flex w-full max-w-280 items-center justify-center gap-5 text-center',
          hiding ? 'animate-insult-out' : 'animate-insult-in',
        ].join(' ')}
      >
        {death.deathStreak > 1 ? (
          <div className="flex min-w-24 shrink-0 flex-col items-center justify-center rounded-md border border-primary/60 bg-primary/15 px-4 py-3">
            <Skull className="text-primary" aria-hidden="true" />
            <span className="font-display text-4xl leading-none">x{death.deathStreak}</span>
          </div>
        ) : null}

        <div className="min-w-0 max-w-full">
          <p className="font-display text-[2.6rem] font-bold uppercase leading-[0.95] tracking-normal text-white wrap-break-word text-pretty drop-shadow-[0_2px_18px_rgba(220,38,38,0.45)]">
            {death.insult}
          </p>
          <p className="mt-2 font-display text-lg uppercase tracking-normal text-red-200/80">
            finished off by {death.killer}
          </p>
        </div>
      </section>
    </main>
  );
}
