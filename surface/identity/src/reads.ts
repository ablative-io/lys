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
