/** A display clock runs only while its connection status is on screen. */
import { useEffect, useState } from 'react';
import type { RefObject } from 'react';

export type Retry = { refused_at: number; next_dial: number; attempt: number; reason: string };

export function retryWords(retry: Retry, now: number): string {
  const lost = new Date(retry.refused_at).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit', second: '2-digit', hour12: true }).toLocaleLowerCase();
  const seconds = Math.max(0, Math.ceil((retry.next_dial - now) / 1000));
  return 'Lost connection to Lys at ' + lost + '. Next try in ' + seconds + ' s (attempt ' + retry.attempt + ').';
}

export function useConnectionClock(moving: boolean, host: RefObject<HTMLDivElement | null>): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    setNow(Date.now());
    const element = host.current;
    if (!moving || element === null) return;
    let visible = false;
    let timer: ReturnType<typeof setInterval> | undefined;
    const stop = () => { clearInterval(timer); timer = undefined; };
    const visibility = () => {
      stop();
      if (visible && document.visibilityState === 'visible') {
        setNow(Date.now());
        timer = setInterval(() => setNow(Date.now()), 1000);
      }
    };
    const observer = new IntersectionObserver((entries) => {
      const entry = entries.find((candidate) => candidate.target === element);
      if (entry) { visible = entry.isIntersecting; visibility(); }
    });
    document.addEventListener('visibilitychange', visibility);
    observer.observe(element);
    return () => { document.removeEventListener('visibilitychange', visibility); observer.disconnect(); stop(); };
  }, [moving, host]);
  return now;
}
