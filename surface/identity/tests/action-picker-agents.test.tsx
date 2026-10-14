import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it, vi } from 'vitest';
import { ActionPicker, appAgentRefusal } from '../src/features/grants/ActionPicker';
import { delegateAgentGrant, grantOptionKey, grantOptions, newAgentGrants } from '../src/features/people/agent-grants';
import { ADA, GRANTS, MODEL, SCRIBE } from './fixtures';

const resource = { kind: 'cambium.channel', id: 'room' };
const model = { ...MODEL, kinds: { 'cambium.channel': {
  agents: ['read', 'send'],
  relations: { reader: ['read'], sender: ['send'], administrator: ['admin'] },
} } };

describe('The app declares what an agent may hold', () => {
  it('sends an approved app delegation and verifies its exact recorded acts', async () => {
    const source = { ...GRANTS[0], resource, actions: ['read', 'send'],
      pass_on: { kind: 'to' as const, actions: ['read', 'send'], recipients: ['agent' as const] } };
    const options = grantOptions({ grants: [source], model, problem: null }, ADA, ADA, 'Ada');
    const [entry] = newAgentGrants(options, [grantOptionKey(options[0])]);
    const body = { operation: entry.operation, route: 'browser' as const, source: entry.source,
      recipient: SCRIBE, responsible: ADA, resource, relation: entry.relation,
      pass_on: { kind: 'use_only' as const }, window: entry.window };
    const id = 'grant-' + 'd'.repeat(32);
    const sent: unknown[] = [];
    vi.stubGlobal('fetch', vi.fn(async (input: string, init?: RequestInit) => {
      let answer: unknown;
      if (input.endsWith('/grants/model')) answer = model;
      else if (input.endsWith('/grants') && init?.method === 'POST') {
        sent.push(JSON.parse(String(init.body)));
        answer = { operation: entry.operation, grant: id, receipt: { caller: ADA } };
      } else if (input.endsWith('/grants/' + id)) {
        answer = { ...source, id, holder: SCRIBE, issuer: ADA, responsible: ADA,
          source: entry.source, operation: entry.operation, relation: entry.relation,
          actions: entry.actions, pass_on: { kind: 'use_only' }, window: entry.window };
      } else throw new Error('Unexpected route: ' + input);
      return new Response(JSON.stringify(answer), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }));
    try {
      await expect(delegateAgentGrant({ ...entry, body }, ADA)).resolves.toBe(id);
      expect(sent).toEqual([body]);
    } finally { vi.unstubAllGlobals(); }
  });
  it('keeps a combined app relation available when the recipient is a person', () => {
    const container = document.createElement('div');
    const root = createRoot(container);
    const personal = { ...model, kinds: { 'cambium.channel': { agents: [], relations: { owner: ['read', 'send'] } } } };
    try {
      act(() => root.render(<ActionPicker model={personal} resource={resource} agents={false}
        actions={['read', 'send']} value={[]} onChange={vi.fn()} />));
      expect(container.querySelector<HTMLInputElement>('input[value="owner"]')?.disabled).toBe(false);
    } finally { act(() => root.unmount()); }
  });
  it('offers declared actions, greys the rest, and selects only declared actions together', () => {
    const container = document.createElement('div');
    const root = createRoot(container);
    const change = vi.fn();
    const choices = ['read', 'send', 'admin'].map((action) => ({
      id: action, resource, relation: action, actions: [action], reason: '',
    }));
    try {
      act(() => root.render(<ActionPicker model={model}
        groups={[{ id: 'room', title: 'The room', choices }]} selected={[]} change={change} />));
      const checkbox = (action: string) => container.querySelector<HTMLInputElement>(`input[value="${action}"]`);
      expect(checkbox('read')?.disabled).toBe(false);
      expect(checkbox('send')?.disabled).toBe(false);
      expect(checkbox('admin')?.disabled).toBe(true);
      expect(container.textContent?.split(appAgentRefusal).length).toBe(2);
      act(() => container.querySelector<HTMLInputElement>('input[name="all_actions"]')?.click());
      expect(change).toHaveBeenCalledWith(['read', 'send']);
    } finally { act(() => root.unmount()); }
  });

  it('uses the app kind\'s own carrier for a single action', () => {
    const container = document.createElement('div');
    const root = createRoot(container);
    try {
      act(() => root.render(<ActionPicker model={model} resource={resource}
        actions={['read', 'send', 'admin']} value={[]} onChange={vi.fn()} />));
      expect(container.querySelector<HTMLInputElement>('input[value="read"]')?.disabled).toBe(false);
      expect(container.querySelector<HTMLInputElement>('input[value="admin"]')?.disabled).toBe(true);
    } finally { act(() => root.unmount()); }
  });

  it('offers a declared combined relation with every act it actually carries', () => {
    const container = document.createElement('div');
    const root = createRoot(container);
    const combined = { ...model, kinds: { 'cambium.channel': { agents: ['read', 'send'], relations: { member: ['read', 'send'] } } } };
    try {
      act(() => root.render(<ActionPicker model={combined} resource={resource}
        actions={['read', 'send']} value={[]} onChange={vi.fn()} />));
      expect(container.querySelector<HTMLInputElement>('input[value="member"]')?.disabled).toBe(false);
      const source = { ...GRANTS[0], resource, relation: 'member', actions: ['read', 'send'],
        pass_on: { kind: 'to' as const, actions: ['read', 'send'], recipients: ['agent' as const] } };
      const options = grantOptions({ grants: [source], model: combined, problem: null }, ADA, ADA, 'Ada');
      expect(options.map((option) => [option.relation, option.actions, option.reason])).toEqual([['member', ['read', 'send'], '']]);
    } finally { act(() => root.unmount()); }
  });

  it('passes the selected app actions from the add-agent form with the exact app relations', () => {
    const source = { ...GRANTS[0], resource, actions: ['read', 'send', 'admin'],
      pass_on: { kind: 'to' as const, actions: ['read', 'send', 'admin'], recipients: ['agent' as const] } };
    const options = grantOptions({ grants: [source], model, problem: null }, ADA, ADA, 'Ada');
    const available = options.filter((option) => !option.reason);
    expect(available.map((option) => [option.relation, option.actions])).toEqual([
      ['reader', ['read']], ['sender', ['send']],
    ]);
    const requests = newAgentGrants(options, available.map(grantOptionKey));
    expect(requests.map((request) => [request.resource, request.relation, request.actions])).toEqual([
      [resource, 'reader', ['read']], [resource, 'sender', ['send']],
    ]);
  });
});
