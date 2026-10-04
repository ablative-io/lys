/** The proxy view of Operations: every agent, the model calls its runs made, and one call whole as JSON. */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, settle, text } from './harness';
import { SCRIBE, SERVICE, dashboard, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { mockTerminal } from './terminal-double';
import { clockOf } from '../src/features/file/time';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const AT = Date.parse('2026-10-04T06:00:00Z');
/** A row of the list, as the service keeps it: a figure the response did not report is null. */
const call = (id: string, more: Record<string, unknown> = {}) => ({ call_id: id, session: 'session-1', at_ms: AT, model: 'claude-fable-5-1', input_tokens: 1200, output_tokens: 340, cache_creation_tokens: 0, cache_read_tokens: 88000,
  account: 'tom', status: 'complete', duration_ms: 2400, ...more });
const lost = call('c2', { status: 'unrecorded', input_tokens: null, output_tokens: null, cache_creation_tokens: null, cache_read_tokens: null, duration_ms: 310, model: null });
const HEAD = { status: 200, request: { names: ['authorization', 'anthropic-version'], values: { 'anthropic-version': ['2023-06-01'] } }, response: { names: ['request-id'], values: { 'request-id': ['req_c1'] } } };
/** A call's record as the proxy keeps it: a member it has nothing for is left out. */
const record = { call_id: 'c1', provider: 'anthropic', api: 'anthropic-messages', model: 'claude-fable-5-1', status: 'complete', stream: true, started_at: '2026-10-04T05:59:57Z', duration_ms: 2400,
  usage: { input: 1200, output: 340, cache_read: 88000 }, request_id: 'req_c1', head: HEAD };
const unrecorded = { call_id: 'c2', provider: 'anthropic', api: 'anthropic-messages', status: 'unrecorded', stream: false, started_at: '2026-10-04T05:59:59Z', duration_ms: 310, unrecorded_reason: 'the response could not be read into parts' };
const request = { model: 'claude-fable-5-1', max_tokens: 32000, messages: [{ role: 'user', content: 'List the files.' }] };
const events = [{ event: 'message_start', data: { type: 'message_start' } }, { event: 'message_stop', data: { type: 'message_stop' } }];
const whole = {
  call: record, request, response: events, request_unreadable: null, response_unreadable: null,
};
const calls = '/agents/' + SCRIBE + '/calls';
const routes: Record<string, Route> = {
  ...SERVICE, '/runtime/live': ok({ sessions: [], unanswered: [] }), '/dashboard': ok(dashboard()),
  [calls]: ok({ agent: SCRIBE, calls: [call('c1'), lost], next: '7.2' }),
  [calls + '?after=7.2']: ok({ agent: SCRIBE, calls: [call('c3', { input_tokens: 5, output_tokens: null, cache_read_tokens: null })], next: null }),
  [calls + '/c1']: ok(whole),
  [calls + '/c2']: ok({ call: unrecorded, request: null, response: null, request_unreadable: 'the body is not JSON: expected value at line 1 column 1', response_unreadable: 'no body was recorded' }),
};

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  localStorage.clear();
});

describe('The proxy view in Operations', () => {
  it('is the other of Operations\' two views: the head swaps between the canvas and the proxy, and the address says which', async () => {
    await mount('#/canvas', routes);
    expect($$('.operations-swap a').map((each) => [each.textContent, each.getAttribute('aria-current')])).toEqual([['Canvas', 'page'], ['Proxy', null]]);
    expect($('.proxy-view')).toBeNull();
    await click($('.operations-swap a[href="#/canvas?view=proxy"]'));
    await settle();
    expect(location.hash).toBe('#/canvas?view=proxy');
    expect($$('.operations-swap a').map((each) => each.getAttribute('aria-current'))).toEqual([null, 'page']);
    expect($$('.proxy-view > section').map((each) => each.getAttribute('aria-label'))).toEqual(['Agents', 'Calls', 'The call']);
    // Every agent the person may see is listed, running or not; nothing is chosen yet, and each column says what to do.
    expect($('[data-agent="' + SCRIBE + '"]')?.textContent).toBe('ScribeNot running');
    expect(text()).toContain('Choose an agent to see the model calls its runs made.');
    expect(text()).toContain('Choose a call to see its request and its response.');
  });

  it('lists an agent\'s calls newest first with what each reported, never a nought for a figure not reported, and reads earlier ones when asked', async () => {
    await mount('#/canvas?view=proxy&agent=' + SCRIBE, routes);
    expect($('[data-agent="' + SCRIBE + '"]')?.getAttribute('aria-current')).toBe('true');
    const cells = (id: string) => [...($('[data-call="' + id + '"]')?.querySelectorAll('td') ?? [])].map((each) => each.textContent);
    expect(cells('c1')).toEqual([clockOf('2026-10-04T06:00:00.000Z'), 'claude-fable-5-1', 'complete', '1,200', '340', '88,000', '2.4 s']);
    // A call that was not recorded is listed all the same and says how it ended; its figures are empty, not zero.
    expect(cells('c2')).toEqual([clockOf('2026-10-04T06:00:00.000Z'), 'Not readable', 'unrecorded', '', '', '', '310 ms']);
    expect($('[data-call="c2"] td.why-not')?.textContent).toBe('unrecorded');
    expect($('[data-call="c1"] a')?.getAttribute('href')).toBe('#/canvas?view=proxy&agent=' + SCRIBE + '&call=c1');
    await click($('[data-act="earlier-calls"]'));
    await settle();
    expect($$('[data-call]').map((each) => each.getAttribute('data-call'))).toEqual(['c1', 'c2', 'c3']);
    expect(cells('c3').slice(3, 6)).toEqual(['5', '', '']);
    expect($('[data-act="earlier-calls"]')).toBeNull();
  });

  it('shows a call whole: its request and its response as the provider\'s JSON set out to be read, and its headers with no credential\'s value', async () => {
    await mount('#/canvas?view=proxy&agent=' + SCRIBE + '&call=c1', routes);
    expect($('[data-call="c1"]')?.getAttribute('aria-current')).toBe('true');
    expect($('.proxy-call-line')?.textContent).toBe('claude-fable-5-1 · complete · HTTP 200 · 2.4 s · streamed · req_c1');
    expect($('[data-json="request"]')?.textContent).toBe(JSON.stringify(request, null, 2));
    expect($$('[data-shows]').map((each) => [each.textContent, each.getAttribute('aria-current')])).toEqual([['Request', 'true'], ['Response', null], ['Headers', null]]);
    await click($('[data-shows="response"]'));
    await settle();
    expect(location.hash).toBe('#/canvas?view=proxy&agent=' + SCRIBE + '&call=c1&shows=response');
    expect($('[data-json="response"]')?.textContent).toBe(JSON.stringify(events, null, 2));
    await click($('[data-shows="headers"]'));
    await settle();
    expect($$('[data-header]').map((each) => each.textContent)).toEqual(['authorizationValue not kept', 'anthropic-version2023-06-01', 'request-idreq_c1']);
    // A body that could not be read as JSON says so with why, and headers that are not on the record are said to be absent.
    await click($('[data-call="c2"] a'));
    await settle();
    expect(text()).toContain('This call has no request that can be read as JSON. the body is not JSON: expected value at line 1 column 1');
    expect($('.proxy-call-line')?.textContent).toBe('Model not readable · unrecorded · 310 ms');
    expect(text()).toContain('the response could not be read into parts');
    await click($('[data-shows="response"]'));
    await settle();
    expect(text()).toContain('This call has no response that can be read as JSON. no body was recorded');
    await click($('[data-shows="headers"]'));
    await settle();
    expect(text()).toContain('No request headers are on the record.');
    expect(text()).toContain('No response headers are on the record.');
  });

  it('says by name what could not be read', async () => {
    await mount('#/canvas?view=proxy&agent=' + SCRIBE + '&call=c9', { ...routes, [calls]: refused(404, 'AgentNotVisible', 'no agent by that id is visible to you'), [calls + '/c9']: refused(404, 'CallUnknown', 'no model call by that id is kept for the agent') });
    expect($('.proxy-view [aria-label="Calls"]')?.textContent).toBe('Its calls could not be read. AgentNotVisible');
    expect($('.proxy-view [aria-label="The call"]')?.textContent).toBe('The call could not be read. CallUnknown');
  });
});
