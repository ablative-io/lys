/** Elapsed time is drawn on animation frames. It never schedules I/O or ends a wait. */
import { useEffect, useState } from 'react';
import { duration } from '../file/time';

export function Elapsed({ since }: { since: number }) {
  const elapsed = () => Math.max(0, Math.floor((Date.now() - since) / 1000));
  const [seconds, setSeconds] = useState(elapsed);
  useEffect(() => {
    if (typeof requestAnimationFrame !== 'function') return;
    let frame: number;
    const draw = () => { setSeconds(elapsed()); frame = requestAnimationFrame(draw); };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  }, [since]);
  return <time dateTime={new Date(since).toISOString()} title={new Date(since).toLocaleString()}>{duration(seconds)}</time>;
}
