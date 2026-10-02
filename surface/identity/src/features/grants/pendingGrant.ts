/** A delegated grant retains its exact request across unknown outcomes and drawer remounts. */
import type { DelegateBody, PassOn } from '../../generated/grants';

type ObjectValue = Record<string, unknown>;
const object = (value: unknown): value is ObjectValue => typeof value === 'object' && value !== null;
const strings = (value: unknown): value is string[] => Array.isArray(value) && value.every((part) => typeof part === 'string');
const second = (value: unknown): value is number => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
export type PendingGrant = { kind: 'empty' } | { kind: 'damaged' } | { kind: 'held'; body: DelegateBody };
export const pendingGrantKey = (person: string, source: string): string => `lys.pending.grant.${person}.${source}`;

export function readPendingGrant(key: string, person: string, source: string): PendingGrant {
  try {
    const raw = sessionStorage.getItem(key);
    if (raw === null) return { kind: 'empty' };
    const body = parsePendingGrant(JSON.parse(raw), person, source);
    return body === null ? { kind: 'damaged' } : { kind: 'held', body };
  } catch {
    return { kind: 'damaged' };
  }
}

/** The grant requests still to send after the retained one, kept beside it under `restKey`; damaged blocks exactly as the retained one does. */
export type PendingRest = { kind: 'empty' } | { kind: 'damaged' } | { kind: 'held'; bodies: DelegateBody[] };
export const restKey = (key: string): string => key + '.rest';

export function readPendingRest(key: string, person: string, source: string): PendingRest {
  try {
    const raw = sessionStorage.getItem(restKey(key));
    if (raw === null) return { kind: 'empty' };
    const value: unknown = JSON.parse(raw);
    if (!Array.isArray(value)) return { kind: 'damaged' };
    const bodies: DelegateBody[] = [];
    for (const each of value) {
      const body = parsePendingGrant(each, person, source);
      if (body === null) return { kind: 'damaged' };
      bodies.push(body);
    }
    return bodies.length ? { kind: 'held', bodies } : { kind: 'empty' };
  } catch {
    return { kind: 'damaged' };
  }
}

/** One retained grant request, exactly as it was sent, or null when the record does not describe one. */
export function parsePendingGrant(value: unknown, person: string, source: string): DelegateBody | null {
  if (!object(value) || typeof value.operation !== 'string' || !/^op-[0-9a-f]{32}$/.test(value.operation)
    || value.route !== 'browser' || value.source !== source || value.responsible !== person
    || typeof value.recipient !== 'string' || typeof value.relation !== 'string'
    || !object(value.resource) || typeof value.resource.kind !== 'string' || typeof value.resource.id !== 'string'
    || !object(value.window) || !second(value.window.starts_at)
    || !(value.window.ends_at === null || second(value.window.ends_at)) || !object(value.pass_on)) return null;
  let passOn: PassOn;
  if (value.pass_on.kind === 'use_only') passOn = { kind: 'use_only' };
  else if (value.pass_on.kind === 'to' && strings(value.pass_on.actions) && strings(value.pass_on.recipients)
    && value.pass_on.recipients.every((recipient) => recipient === 'person' || recipient === 'agent')) {
    passOn = { kind: 'to', actions: value.pass_on.actions, recipients: value.pass_on.recipients as ('person' | 'agent')[] };
  } else return null;
  return {
    operation: value.operation, route: 'browser', source, responsible: person, recipient: value.recipient,
    relation: value.relation, resource: { kind: value.resource.kind, id: value.resource.id },
    window: { starts_at: value.window.starts_at, ends_at: value.window.ends_at }, pass_on: passOn,
  };
}
