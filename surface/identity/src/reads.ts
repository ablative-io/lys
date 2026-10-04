import { request } from './api';

/** Independent reads begin together; their named results keep their types. */
export async function readTogether<T extends Record<string, Promise<unknown>>>(reads: T): Promise<{ [K in keyof T]: Awaited<T[K]> }> {
  const entries = await Promise.all(Object.entries(reads).map(async ([key, value]) => [key, await value]));
  return Object.fromEntries(entries) as { [K in keyof T]: Awaited<T[K]> };
}

/** A collection never opens more than four reads at once. */
export async function readBounded<T, R>(values: readonly T[], read: (value: T) => Promise<R>): Promise<R[]> {
  const results = new Array<R>(values.length);
  let next = 0;
  let failed = false;
  await Promise.all(Array.from({ length: Math.min(4, values.length) }, async () => {
    while (!failed && next < values.length) {
      const index = next++;
      try { results[index] = await read(values[index]); }
      catch (error) { failed = true; throw error; }
    }
  }));
  return results;
}

/** One page of a list the service pages: its rows under their own member, and the cursor of the next page, null on the last. */
export interface Paged { total?: number; next?: string | null }

/**
 * A paged list read whole: each next page in turn, after the last, until a page's `next` is null, so nothing past the
 * first page is left unread. `join` adds a page's rows to what was read so far. A cursor that comes round again is
 * refused rather than read twice.
 */
export async function readEveryPage<P extends Paged>(path: string, join: (read: P, page: P) => P): Promise<P> {
  let read = await request<P>(path);
  const seen = new Set<string>();
  while (typeof read.next === 'string') {
    if (seen.has(read.next)) throw new Error('PagingRepeated: the service gave the same next page twice for ' + path + '.');
    seen.add(read.next);
    const page = await request<P>(path + (path.includes('?') ? '&' : '?') + 'after=' + encodeURIComponent(read.next));
    read = { ...join(read, page), next: page.next ?? null };
  }
  return read;
}
