/** Message pages preserve authoritative addressing, scope and completeness without terminal inference. */
import { describe, expect, it, vi } from 'vitest';
import { firstMessagePage, readMessagePage } from '../src/features/runtime/message-connections';
import { withMessages } from '../src/features/runtime/session-graph';

const blank = { places: [], messages: [], roots: [], next: null, unmapped: [] };
const edge = { message: 'post-1', stream: 'chat', source: 'person-a', recipients: ['agent-b'], addressing: 'direct' as const, at: 1 };

function service(pages: Record<string, unknown>) {
  const reads: string[] = [];
  vi.stubGlobal('fetch', async (input: string) => {
    reads.push(input);
    if (!(input in pages)) throw new Error('Unexpected request: ' + input);
    return new Response(JSON.stringify(pages[input]), { status: 200 });
  });
  return reads;
}
const base = '/api/runtime/message-edges';

describe('Message connections', () => {
  it('pages places, posts and replies without silently calling the first page complete', async () => {
    const reads = service({
      [base]: { ...blank, places: ['chat'], next: 'places-2' },
      [base + '?stream=chat']: { ...blank, messages: [edge], roots: ['post-1'], next: 'posts-2' },
      [base + '?cursor=places-2']: blank,
      [base + '?stream=chat&root=post-1']: { ...blank, messages: [{ ...edge, message: 'reply-1' }] },
      [base + '?stream=chat&cursor=posts-2']: { ...blank, messages: [edge, { ...edge, message: 'post-2' }] },
    });
    let state = await firstMessagePage();
    expect(state.messages).toEqual([edge]);
    expect(state.pending).toHaveLength(3);
    state = await readMessagePage(state);
    state = await readMessagePage(state);
    state = await readMessagePage(state);
    expect(state.pending).toEqual([]);
    expect(state.messages.map((message) => message.message)).toEqual(['post-1', 'reply-1', 'post-2']);
    expect(reads).toHaveLength(5);
  });

  it('refuses a repeated cursor at the response that repeats it', async () => {
    service({ [base + '?stream=chat&cursor=repeat']: { ...blank, next: 'repeat' } });
    await expect(readMessagePage({ messages: [], pending: [{ stream: 'chat', cursor: 'repeat' }], seen: [], unmapped: [] })).rejects.toThrow('repeated a page');
  });

  it('refuses evidence returned for another stream and malformed identities', async () => {
    service({ [base + '?stream=other']: { ...blank, messages: [edge] } });
    await expect(readMessagePage({ messages: [], pending: [{ stream: 'other' }], seen: [], unmapped: [] })).rejects.toThrow('invalid addressing evidence');
    service({ [base]: { ...blank, unmapped: [42] } });
    await expect(firstMessagePage()).rejects.toThrow('unreadable page');
  });

  it('draws message identity edges, never claims the terminal consumed them', () => {
    const graph = withMessages({ nodes: [], edges: [], names: { 'agent-b': 'Scribe' }, notices: [], unanswered: [] }, [edge]);
    expect(graph.edges).toEqual([{ id: 'message:post-1:agent-b', from: 'identity:person-a', to: 'identity:agent-b', kind: 'message', stands: true, label: 'Direct message post-1 in chat' }]);
    expect(graph.nodes.every((node) => node.session === undefined)).toBe(true);
    expect(graph.nodes.map((node) => node.title)).toContain('Scribe');
  });
});
