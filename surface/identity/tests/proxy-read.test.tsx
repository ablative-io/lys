/** A call read on the proxy view: what went in and what came out, in words, with nothing guessed. */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, mount } from './harness';
import { SCRIBE, SERVICE, dashboard, ok } from './fixtures';
import type { Route } from './fixtures';
import { mockTerminal } from './terminal-double';
import { readIn, readOut } from '../src/features/proxy/proxy-read';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const request = {
  model: 'claude-fable-5-1', max_tokens: 32000, stream: true, thinking: { type: 'adaptive' }, output_config: { effort: 'high' },
  system: [{ type: 'text', text: 'You are Scribe.' }, { type: 'text', text: 'Be brief.' }],
  tools: [{ name: 'Bash', description: 'Runs a command' }, { name: 'Read' }],
  messages: [
    { role: 'user', content: 'List the files.' },
    { role: 'assistant', content: [{ type: 'thinking', thinking: '' }, { type: 'text', text: 'Looking.' }, { type: 'tool_use', id: 't1', name: 'Bash', input: { command: 'ls' } }] },
    { role: 'user', content: [{ type: 'tool_result', tool_use_id: 't1', content: [{ type: 'text', text: 'a.txt' }, { type: 'text', text: 'b.txt' }] }, { type: 'image', source: { type: 'base64' } }] },
  ],
};
const event = (type: string, data: Record<string, unknown> = {}) => ({ event: type, data: { type, ...data } });
const events = [
  event('message_start', { message: { role: 'assistant', content: [] } }),
  event('content_block_start', { index: 0, content_block: { type: 'text', text: '' } }), event('ping'),
  event('content_block_delta', { index: 0, delta: { type: 'text_delta', text: 'Two files: ' } }), event('content_block_delta', { index: 0, delta: { type: 'text_delta', text: 'a and b.' } }),
  event('content_block_stop', { index: 0 }),
  event('content_block_start', { index: 1, content_block: { type: 'tool_use', id: 't2', name: 'Read', input: {} } }),
  event('content_block_delta', { index: 1, delta: { type: 'input_json_delta', partial_json: '{"file_path":' } }), event('content_block_delta', { index: 1, delta: { type: 'input_json_delta', partial_json: '"a.txt"}' } }),
  event('message_delta', { delta: { stop_reason: 'tool_use' } }), event('message_stop'),
];

describe('A call, read apart', () => {
  it('reads a request into its settings, its system prompt, its tools by name and each turn\'s pieces in order', () => {
    const went = readIn(request);
    expect(went?.settings).toEqual([['Model', 'claude-fable-5-1'], ['Most tokens out', '32000'], ['Streamed', 'true'], ['Thinking', 'adaptive'], ['Effort', 'high']]);
    expect(went?.system.map((each) => each.text)).toEqual(['You are Scribe.', 'Be brief.']);
    expect(went?.tools).toEqual(['Bash', 'Read']);
    expect(went?.turns.map((turn) => [turn.role, turn.pieces.map((each) => each.kind)])).toEqual([['user', ['text']], ['assistant', ['thinking', 'text', 'tool']], ['user', ['result', 'other']]]);
    expect(went?.turns[1].pieces[2]).toMatchObject({ label: 'Bash', json: { command: 'ls' } });
    // A tool's result given as parts of text is read as its lines; a part this view does not know is kept whole as JSON.
    expect(went?.turns[2].pieces[0]).toMatchObject({ label: 'Tool result', text: 'a.txt\nb.txt' });
    expect(went?.turns[2].pieces[1]).toMatchObject({ kind: 'other', label: 'image', json: { type: 'image', source: { type: 'base64' } } });
    // A request with only what it must have reads with nothing put in for what it left out.
    expect(readIn({ messages: [] })).toEqual({ settings: [], system: [], turns: [], tools: [] });
  });

  it('puts a streamed response back together from its events, and reads one given whole and an error as they are', () => {
    const came = readOut(events);
    expect(came?.pieces.map((each) => [each.kind, each.label, each.text, each.json === undefined ? null : each.kind === 'tool' ? each.json : null])).toEqual([['text', 'Text', 'Two files: a and b.', null], ['tool', 'Read', null, { file_path: 'a.txt' }]]);
    expect([came?.stop, came?.error]).toEqual(['tool_use', null]);
    expect(readOut({ type: 'message', role: 'assistant', stop_reason: 'end_turn', content: [{ type: 'text', text: 'Done.' }] })).toMatchObject({ stop: 'end_turn', error: null, pieces: [{ kind: 'text', text: 'Done.' }] });
    expect(readOut({ type: 'error', error: { type: 'rate_limit_error', message: 'Error' }, request_id: 'req_1' })).toEqual({ pieces: [], stop: null, error: { type: 'rate_limit_error', message: 'Error' } });
    // A tool's input cut off mid-stream is kept as the text that arrived, never mended.
    expect(readOut([events[0], events[6], events[7]])?.pieces[0]).toMatchObject({ kind: 'tool', json: '{"file_path":' });
  });

  it('answers nothing for a body that is not in a shape it reads, and guesses at none', () => {
    expect([readIn(null), readIn({ input: 'a Responses call' }), readIn('words')]).toEqual([null, null, null]);
    expect([readOut(null), readOut({ id: 'resp_1', output: [] }), readOut([{ event: 'response.created', data: { type: 'response.created' } }]), readOut([])]).toEqual([null, null, null, null]);
  });
});

const AT = Date.parse('2026-10-04T06:00:00Z');
const row = (id: string, status: string) => ({ call_id: id, session: 'session-1', at_ms: AT, model: 'claude-fable-5-1', input_tokens: 1, output_tokens: 1, cache_creation_tokens: 0, cache_read_tokens: 0, account: 'tom', status, duration_ms: 10 });
const kept = (id: string, status: string) => ({ call_id: id, provider: 'anthropic', api: 'anthropic-messages', model: 'claude-fable-5-1', status, stream: true, started_at: '2026-10-04T05:59:57Z', duration_ms: 10 });
const calls = '/agents/' + SCRIBE + '/calls';
const routes: Record<string, Route> = {
  ...SERVICE, '/runtime/live': ok({ sessions: [], unanswered: [] }), '/dashboard': ok(dashboard()),
  [calls]: ok({ agent: SCRIBE, calls: [row('c1', 'complete'), row('c2', 'unrecorded'), row('c3', 'complete')], next: null }),
  [calls + '/c1']: ok({ call: kept('c1', 'complete'), request, response: events, request_unreadable: null, response_unreadable: null }),
  [calls + '/c2']: ok({ call: { ...kept('c2', 'unrecorded'), head: { status: 429, request: { names: [], values: {} }, response: { names: [], values: {} } } }, request: { model: 'claude-fable-5-1', max_tokens: 1, messages: [{ role: 'user', content: 'quota' }] },
    response: { type: 'error', error: { type: 'rate_limit_error', message: 'Error' } }, request_unreadable: null, response_unreadable: null }),
  [calls + '/c3']: ok({ call: kept('c3', 'complete'), request: { input: 'a Responses call' }, response: { id: 'resp_1', output: [] }, request_unreadable: null, response_unreadable: null }),
};

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  localStorage.clear();
});

describe('A call opened on the proxy view', () => {
  it('opens read: what went in with the last turn open, then what came out and why it stopped', async () => {
    await mount('#/canvas?view=proxy&agent=' + SCRIBE + '&call=c1', routes);
    expect($$('[data-shows]').map((each) => [each.textContent, each.getAttribute('aria-current')])).toEqual([['Read', 'true'], ['Request', null], ['Response', null], ['Headers', null]]);
    const went = $('[data-read="in"]');
    expect(went?.querySelector('.proxy-call-line')?.textContent).toBe('Model: claude-fable-5-1 · Most tokens out: 32000 · Streamed: true · Thinking: adaptive · Effort: high');
    expect(went?.querySelector('[data-part="system"] summary')?.textContent).toBe('System prompt 2 parts · 24 characters');
    expect(went?.querySelector('[data-part="tools"]')?.textContent).toBe('Tools offered 2Bash, Read');
    const turns = [...(went?.querySelectorAll('[data-part="turn"]') ?? [])] as HTMLDetailsElement[];
    expect(turns.map((each) => [each.querySelector('summary')?.textContent, each.open])).toEqual([
      ['1. user 15 characters', false], ['2. assistant 8 characters · asks for Bash', false], ['3. user 11 characters · 1 tool result', true]]);
    expect(turns[1].querySelector('[data-piece="thinking"]')?.textContent).toBe('Thinking, which the provider did not show.');
    expect(turns[1].querySelector('[data-piece="tool"]')?.textContent).toBe('Tool asked for: Bash' + JSON.stringify({ command: 'ls' }, null, 2));
    expect(turns[2].querySelector('[data-piece="result"]')?.textContent).toBe('Tool resulta.txt\nb.txt');
    const came = $('[data-read="out"]');
    expect(came?.querySelector('.proxy-words')?.textContent).toBe('Two files: a and b.');
    expect(came?.querySelector('[data-piece="tool"]')?.textContent).toBe('Tool asked for: Read' + JSON.stringify({ file_path: 'a.txt' }, null, 2));
    expect(came?.querySelector('[data-part="stop"]')?.textContent).toBe('Stopped: tool_use');
  });

  it('says the provider\'s error by its own name for a call it refused, and points at the JSON for a shape it does not set out', async () => {
    await mount('#/canvas?view=proxy&agent=' + SCRIBE + '&call=c2', routes);
    expect($('.proxy-call-line')?.textContent).toContain('unrecorded · HTTP 429');
    expect($('[data-read="out"] [data-part="error"]')?.textContent).toBe('The provider answered an error: rate_limit_error, "Error".');
    expect($('[data-read="in"] [data-part="turn"] .proxy-words')?.textContent).toBe('quota');
    await mount('#/canvas?view=proxy&agent=' + SCRIBE + '&call=c3', routes);
    expect($('[data-read="in"]')?.textContent).toContain('This request is not in a shape this view sets out. It is whole under Request.');
    expect($('[data-read="out"]')?.textContent).toContain('This response is not in a shape this view sets out. It is whole under Response.');
  });
});
