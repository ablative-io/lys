/**
 * What a directory receipt's change was, in words, read from the signed event
 * the receipt answer carries (`message`: the log leaf, hex).
 *
 * The leaf is a COSE_Sign1 message (tag 18, an array of four) whose payload is
 * the event body: a canonical CBOR map, key 6 the change kind and key 7 the
 * change (crates/lys-identity/src/encoding.rs, `encode_body` and `change`). A
 * lifecycle move carries its transition, the state it left, the state it
 * entered and its reason (keys 1 to 4); a profile change carries the new name.
 * Nothing here verifies the signature: the words only describe the change, and
 * the entry link beside each line is the evidence. Anything that cannot be read
 * falls back to the change kind's own words; this never throws.
 */
import { CHANGE_KINDS } from '../../generated';
import type { ReceiptAnswer } from '../../generated';

type Cbor = number | string | Uint8Array | Cbor[] | Map<Cbor, Cbor> | { tag: number; value: Cbor } | boolean | null;

function bytesOf(hex: string): Uint8Array | null {
  if (!/^(?:[0-9a-f]{2})*$/i.test(hex) || !hex.length) return null;
  const out = new Uint8Array(hex.length / 2);
  for (let at = 0; at < out.length; at += 1) out[at] = parseInt(hex.slice(at * 2, at * 2 + 2), 16);
  return out;
}

/** A decoder for the definite-length subset the directory writes; null on anything else. */
function decode(data: Uint8Array): Cbor | null {
  let at = 0;
  const length = (info: number): number | null => {
    if (info < 24) return info;
    const size = info === 24 ? 1 : info === 25 ? 2 : info === 26 ? 4 : info === 27 ? 8 : 0;
    if (!size || at + size > data.length) return null;
    let value = 0;
    for (let index = 0; index < size; index += 1) value = value * 256 + data[at + index];
    at += size;
    return Number.isSafeInteger(value) ? value : null;
  };
  const item = (depth: number): Cbor | undefined => {
    if (depth > 16 || at >= data.length) return undefined;
    const first = data[at]; at += 1;
    const major = first >> 5;
    const info = first & 31;
    if (major === 7) return info === 20 ? false : info === 21 ? true : info === 22 ? null : undefined;
    const n = length(info);
    if (n === null) return undefined;
    switch (major) {
      case 0: return n;
      case 1: return -1 - n;
      case 2: case 3: {
        if (at + n > data.length) return undefined;
        const slice = data.slice(at, at + n); at += n;
        return major === 2 ? slice : new TextDecoder().decode(slice);
      }
      case 4: {
        const list: Cbor[] = [];
        for (let index = 0; index < n; index += 1) { const next = item(depth + 1); if (next === undefined) return undefined; list.push(next); }
        return list;
      }
      case 5: {
        const map = new Map<Cbor, Cbor>();
        for (let index = 0; index < n; index += 1) {
          const key = item(depth + 1); const value = item(depth + 1);
          if (key === undefined || value === undefined) return undefined;
          map.set(key, value);
        }
        return map;
      }
      case 6: { const value = item(depth + 1); return value === undefined ? undefined : { tag: n, value }; }
      default: return undefined;
    }
  };
  const value = item(0);
  return value === undefined || at !== data.length ? null : value;
}

const STATES: Record<number, string> = { 1: 'registered', 2: 'active', 3: 'suspended', 4: 'retired' };
const MOVES: Record<number, string> = { 1: 'Switched on', 2: 'Suspended', 3: 'Restored', 4: 'Retired' };
/** Change kinds the generated table does not name yet (crates/lys-identity/src/event.rs, `wire`). */
const MORE_KINDS: Record<number, string> = { 8: 'registered, answering to', 9: 'reporting changed', 10: 'a change it made through Lys' };

const hexOf = (value: Uint8Array) => [...value].map((byte) => byte.toString(16).padStart(2, '0')).join('');

/** The event body inside a leaf, or null. */
function body(message: string): Map<Cbor, Cbor> | null {
  const raw = bytesOf(message);
  if (!raw) return null;
  let value = decode(raw);
  if (value && typeof value === 'object' && 'tag' in value) value = value.value;
  if (!Array.isArray(value) || value.length !== 4 || !(value[2] instanceof Uint8Array)) return null;
  const inner = decode(value[2]);
  return inner instanceof Map ? inner : null;
}

/** A reporting target, `{1: kind, 2: id}`, as an identity id. */
function target(value: Cbor | undefined): string | null {
  if (!(value instanceof Map) || !(value.get(2) instanceof Uint8Array)) return null;
  const kind = value.get(1);
  const id = value.get(2) as Uint8Array;
  return kind === 1 ? 'person-' + hexOf(id) : kind === 2 ? 'agent-' + hexOf(id) : null;
}

/** What a receipt's change was. `from` and `to` are said when the event carries them; `names` are identity ids to be named by the caller. */
export interface ChangeWords { words: string; reason: string; names: { from: string | null; to: string | null } }

export function changeWords(answer: ReceiptAnswer): ChangeWords {
  const kind = answer.receipt.change_kind;
  const plain = { words: CHANGE_KINDS[kind] ?? MORE_KINDS[kind] ?? 'change ' + kind, reason: '', names: { from: null, to: null } };
  const event = typeof answer.message === 'string' ? body(answer.message) : null;
  const change = event?.get(7);
  if (!event || event.get(6) !== kind || !(change instanceof Map)) return plain;
  if (kind === 5) {
    const move = change.get(1); const from = change.get(2); const to = change.get(3); const reason = change.get(4);
    if (typeof move !== 'number' || typeof from !== 'number' || typeof to !== 'number' || !MOVES[move] || !STATES[from] || !STATES[to]) return plain;
    return { words: MOVES[move] + ': ' + STATES[from] + ' → ' + STATES[to], reason: typeof reason === 'string' ? reason.trim() : '', names: { from: null, to: null } };
  }
  if (kind === 3 || kind === 7 || kind === 1) {
    const profile = change.get(1);
    const name = profile instanceof Map ? profile.get(1) : undefined;
    if (typeof name !== 'string') return plain;
    return { words: kind === 3 ? 'Name changed to ' + name : kind === 1 ? 'Registered as ' + name : 'Setup completed for ' + name, reason: '', names: { from: null, to: null } };
  }
  if (kind === 9) {
    const from = target(change.get(1)); const to = target(change.get(2));
    if (!from || !to) return plain;
    return { words: 'Now answers to', reason: '', names: { from, to } };
  }
  return plain;
}
