import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Delegate } from '../src/features/grants/Delegate';
import type { GrantWorld } from '../src/features/grants/model';
import type { DelegateBody, Grant, GrantModel } from '../src/generated/grants';
import { ShellProvider } from '../src/shell/ShellContext';
import { ADA, DIRECTORY, GRANTS, ME, MODEL, SCRIBE, SCRIBE_G, ok } from './fixtures';
import { serve } from './harness';

let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { if (root) act(() => root?.unmount()); root = null; vi.unstubAllGlobals(); });

const resource = { kind: 'cambium.channel', id: 'room' };
const model: GrantModel = { ...MODEL, kinds: { 'cambium.channel': {
  agents: ['read', 'send'], relations: { reader: ['read'], sender: ['send'], participant: ['read', 'send'], administrator: ['admin'] },
} } };
const source: Grant = { ...GRANTS[0], resource, relation: 'participant', actions: ['read', 'send'],
  pass_on: { kind: 'to', actions: ['read', 'send'], recipients: ['agent'] } };

async function open(held = source, metadata = model) {
  const posted: { path: string; body: unknown }[] = [];
  const done = vi.fn();
  serve({
    '/roles': ok({ roles: [] }),
    [`/grants/cannot-give?route=browser&source=${held.id}&recipient=${SCRIBE}`]: ok({ source: held.id, recipient: SCRIBE, items: [] }),
    'POST /grants': (body) => ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, receipt: { caller: ADA } }),
  }, posted);
  const world: GrantWorld = { me: ME, people: DIRECTORY, list: { grants: [held], revision: 7 }, model: metadata,
    byId: new Map([[held.id, held]]), who: new Map([
      [ADA, { name: 'Ada', state: 'active', kind: 'person', responsible: null }],
      [SCRIBE, { name: 'Scribe', state: 'active', kind: 'agent', responsible: ADA }],
    ]) };
  const container = document.createElement('div'); document.body.appendChild(container); root = createRoot(container);
  await act(async () => { root?.render(<ShellProvider><Delegate w={world} source={held} to={SCRIBE} done={done} close={vi.fn()} /></ShellProvider>); });
  return { container, posted, done };
}

describe('Delegating approved app actions', () => {
  it('offers Cambium read and send using its own single-action carriers and sends only the ticked action', async () => {
    const { container, posted, done } = await open();
    const offered = [...container.querySelectorAll<HTMLInputElement>('input[name="action"]')];
    expect(offered.map((input) => input.value)).toEqual(['read', 'send']);
    expect(offered.every((input) => !input.disabled)).toBe(true);
    expect(container.textContent).not.toContain('until the app allows it');
    await act(async () => { offered[0].click(); });
    await act(async () => { container.querySelector<HTMLButtonElement>('[data-act="delegatedo"]')?.click(); });
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ path: '/grants', body: { resource, relation: 'reader', source: source.id, recipient: SCRIBE, pass_on: { kind: 'use_only' } } });
    expect(done).toHaveBeenCalledOnce();
  });

  it('offers an app-declared combined participant relation with exactly read and send when no single-action carrier exists', async () => {
    const combined = { ...model, kinds: { 'cambium.channel': { agents: ['read', 'send'], relations: { participant: ['read', 'send'] } } } };
    const { container, posted } = await open(source, combined);
    const offered = [...container.querySelectorAll<HTMLInputElement>('input[name="action"]')];
    expect(offered.map((input) => input.value)).toEqual(['relation:participant']);
    expect(offered[0].closest('tr')?.querySelector('label')?.textContent).toBe('read; send');
    expect(offered[0].disabled).toBe(false);
    expect(container.textContent).not.toContain('Some actions cannot be selected');
    await act(async () => { offered[0].click(); });
    await act(async () => { container.querySelector<HTMLButtonElement>('[data-act="delegatedo"]')?.click(); });
    expect(posted).toHaveLength(1);
    expect(posted[0].body).toMatchObject({ resource, relation: 'participant', pass_on: { kind: 'use_only' } });
  });

  it('says the holder has nothing an agent could be given and states the held actions when Cambium permits other agent actions', async () => {
    const admin: Grant = { ...source, relation: 'administrator', actions: ['admin'], pass_on: { kind: 'to', actions: ['admin'], recipients: ['agent'] } };
    const { container } = await open(admin);
    expect(container.textContent).toContain('You hold nothing on this resource that an agent could be given');
    expect(container.textContent).toContain('You hold: admin.');
    expect(container.textContent).not.toContain('until the app allows it');
    expect(container.querySelector('input[name="action"]')).toBeNull();
  });

  it('states what is held when none of it may be passed on', async () => {
    const { container } = await open({ ...source, pass_on: { kind: 'use_only' } });
    expect(container.textContent).toContain('You hold nothing on this resource that an agent could be given');
    expect(container.textContent).toContain('You hold: read; send.');
    expect(container.textContent).not.toContain('until the app allows it');
    expect(container.querySelector<HTMLButtonElement>('[data-act="delegatedo"]')?.disabled).toBe(true);
  });

  it('says until the app allows it only when the served kind declares no agent actions', async () => {
    const disallowed = { ...model, kinds: { 'cambium.channel': { ...model.kinds['cambium.channel'], agents: [] } } };
    const { container } = await open(source, disallowed);
    expect(container.textContent).toContain("Lys can't give an agent this app's actions until the app allows it.");
    expect(container.querySelector('input[name="action"]')).toBeNull();
  });

  it('names an app kind Lys has not declared beside the grant and renders the rest of the form', async () => {
    const { container, posted } = await open({ ...source, resource: { kind: 'retired.app', id: 'room' } });
    expect(container.textContent).toContain('Lys has not declared the app kind retired.app; an administrator approves the app under Apps.');
    expect(container.textContent).toContain('Where it comes from');
    expect(container.querySelector('input[name="action"]')).toBeNull();
    expect(container.querySelector<HTMLButtonElement>('[data-act="delegatedo"]')?.disabled).toBe(true);
    expect(posted).toEqual([]);
  });

  it('keeps an app relation named like an action apart from that action, so ticking the action sends its single carrier', async () => {
    const clash = { ...model, kinds: { 'cambium.channel': { agents: ['read', 'send'], relations: { read: ['read', 'send'], reader: ['read'] } } } };
    const { container, posted } = await open(source, clash);
    const offered = [...container.querySelectorAll<HTMLInputElement>('input[name="action"]')];
    expect(offered.map((input) => input.value)).toEqual(['read', 'relation:read']);
    await act(async () => { offered[0].click(); });
    await act(async () => { container.querySelector<HTMLButtonElement>('[data-act="delegatedo"]')?.click(); });
    expect(posted).toHaveLength(1);
    expect(posted[0].body).toMatchObject({ resource, relation: 'reader' });
  });

  it('names the unavailable carrier when only part of a combined app relation may be passed on', async () => {
    const combined = { ...model, kinds: { 'cambium.channel': { agents: ['read', 'send'], relations: { participant: ['read', 'send'] } } } };
    const limited: Grant = { ...source, actions: ['read'], pass_on: { kind: 'to', actions: ['read'], recipients: ['agent'] } };
    const { container, posted } = await open(limited, combined);
    expect(container.textContent).toContain('The app has no relation carrying only the agent actions you may pass on.');
    expect(container.textContent).not.toContain('until the app allows it');
    expect(container.querySelector('input[name="action"]')).toBeNull();
    expect(posted).toEqual([]);
  });
});
