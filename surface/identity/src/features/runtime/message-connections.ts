/** Paged message evidence from Cambium through Lys; cursors are scoped, and reads never imply delivery to a terminal. */
import { request } from '../../api';

export interface MessageConnection {
  message: string; stream: string; source: string; recipients: string[];
  addressing: 'direct' | 'mentioned'; at: number;
}
interface Scope { stream?: string; root?: string; cursor?: string }
interface Page { places: string[]; messages: MessageConnection[]; roots: string[]; next: string | null; unmapped: string[] }
export interface MessageRead { messages: MessageConnection[]; pending: Scope[]; seen: string[]; unmapped: string[] }

export async function readMessagePage(previous: MessageRead): Promise<MessageRead> {
  const [scope, ...pending] = previous.pending;
  if (!scope) return previous;
  const key = JSON.stringify(scope);
  if (previous.seen.includes(key)) throw new Error('Message pagination repeated a page; refresh before continuing.');
  const query = new URLSearchParams();
  if (scope.stream !== undefined) query.set('stream', scope.stream);
  if (scope.root !== undefined) query.set('root', scope.root);
  if (scope.cursor !== undefined) query.set('cursor', scope.cursor);
  const page = await request<Page>('/runtime/message-edges' + (query.size ? '?' + query.toString() : ''));
  if (!Array.isArray(page.places) || !Array.isArray(page.messages) || !Array.isArray(page.roots) || !Array.isArray(page.unmapped)
    || !page.unmapped.every((id) => typeof id === 'string')
    || (page.next !== null && typeof page.next !== 'string')) throw new Error('The message service returned an unreadable page.');
  for (const message of page.messages) {
    if (typeof message.message !== 'string' || typeof message.stream !== 'string' || typeof message.source !== 'string'
      || !Array.isArray(message.recipients) || !message.recipients.every((id) => typeof id === 'string')
      || !['direct', 'mentioned'].includes(message.addressing) || !Number.isSafeInteger(message.at) || message.at < 0
      || message.stream !== scope.stream) throw new Error('The message service returned invalid addressing evidence.');
  }
  for (const stream of page.places) {
    if (typeof stream !== 'string' || scope.stream !== undefined) throw new Error('The message service returned places in the wrong scope.');
    pending.push({ stream });
  }
  for (const root of page.roots) {
    if (typeof root !== 'string' || !scope.stream || scope.root) throw new Error('The message service returned thread roots in the wrong scope.');
    pending.push({ stream: scope.stream, root });
  }
  if (page.next !== null) {
    const continuation = { ...scope, cursor: page.next };
    if ([...previous.seen, key].includes(JSON.stringify(continuation))) throw new Error('Message pagination repeated a page; refresh before continuing.');
    pending.push(continuation);
  }
  const messages = new Map(previous.messages.map((entry) => [entry.message, entry]));
  for (const entry of page.messages) messages.set(entry.message, entry);
  return { messages: [...messages.values()], pending, seen: [...previous.seen, key], unmapped: [...new Set([...previous.unmapped, ...page.unmapped])] };
}

export async function firstMessagePage(): Promise<MessageRead> {
  const places = await readMessagePage({ messages: [], pending: [{}], seen: [], unmapped: [] });
  return places.pending.length ? readMessagePage(places) : places;
}
