/** Required body members derived from the server's own request schemas. */
import { afterEach, beforeEach, expect } from 'vitest';
import contracts from './server-required-fields.json';

const failures: string[] = [];
const routes = Object.entries(contracts).map(([route, fields]) => ({
  method: route.slice(0, route.indexOf(' ')),
  path: new RegExp('^' + route.slice(route.indexOf(' ') + 1).replace(/\{[^}]+\}/g, '[^/?#]+') + '$'),
  fields,
}));

export function missingFields(method: string, path: string, body: unknown): string[] {
  const route = routes.find((entry) => entry.method === method && entry.path.test(path.split('?')[0]));
  if (!route) return [];
  const record = body && typeof body === 'object' && !Array.isArray(body) ? body as Record<string, unknown> : {};
  return route.fields.filter((field) => !Object.hasOwn(record, field) || record[field] === undefined);
}

export function checkServerRequest(method: string, path: string, body: unknown): void {
  const missing = missingFields(method, path, body);
  if (missing.length) failures.push(method + ' ' + path + ' missing required server fields: ' + missing.join(', '));
}

beforeEach(() => { failures.length = 0; });
afterEach(() => { expect(failures, 'Surface bodies must satisfy the server request schemas').toEqual([]); });
