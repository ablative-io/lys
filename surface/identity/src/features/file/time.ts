const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** Seconds since the epoch as the mock-up writes a date, `22 Sep`. */
export const day = (seconds: number): string => {
  const d = new Date(seconds * 1000);
  return `${d.getDate()} ${MONTHS[d.getMonth()]}`;
};

/** A date and the minute, `22 Sep 09:14`. */
export const clock = (seconds: number): string => {
  const d = new Date(seconds * 1000);
  return `${day(seconds)} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
};

/** The same, from milliseconds or from a written date. Every screen's dates come from this file, in the reader's own time. */
export const clockMs = (ms: number): string => clock(Math.floor(ms / 1000));
export const clockOf = (written: string): string => clockMs(Date.parse(written));

/** A length of time in words: `8 hours`, `1 hour 30 minutes`, never a count of seconds alone unless it is under a minute. */
export function duration(seconds: number): string {
  const parts: [number, string][] = [[Math.floor(seconds / 86400), 'day'], [Math.floor(seconds % 86400 / 3600), 'hour'], [Math.floor(seconds % 3600 / 60), 'minute'], [Math.floor(seconds % 60), 'second']];
  const said = parts.filter(([count]) => count > 0).map(([count, unit]) => count + ' ' + unit + (count === 1 ? '' : 's'));
  return said.length ? said.join(' ') : '0 seconds';
}

/** How long ago, as a person says it. */
export function ago(seconds: number): string {
  const gone = Math.max(0, Math.floor(Date.now() / 1000) - seconds);
  if (gone < 60) return 'just now';
  if (gone < 3600) return Math.floor(gone / 60) + ' min ago';
  if (gone < 86400) return Math.floor(gone / 3600) + ' h ago';
  return Math.floor(gone / 86400) + ' d ago';
}
