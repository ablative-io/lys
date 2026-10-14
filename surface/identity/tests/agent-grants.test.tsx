import { afterEach, describe, expect, it, vi } from 'vitest';
import { grantOptionKey, grantOptions, newAgentGrants, readGrantChoices } from '../src/features/people/agent-grants';
import type { GrantChoices } from '../src/features/people/agent-grants';
import { appAgentRefusal } from '../src/features/grants/ActionPicker';
import { ADA, GRANTS, MODEL, ok } from './fixtures';
import { serve } from './harness';

afterEach(() => vi.unstubAllGlobals());

const source = GRANTS[0];
const model = { ...MODEL, relations: { ...MODEL.relations, 'only.edit': ['edit'], 'only.view': ['view'], 'only.grant': ['grant'] }, withheld_from_agents: ['grant'] };
const wide = { ...source, pass_on: { kind: 'to' as const, actions: ['edit', 'grant', 'view'], recipients: ['agent' as const] } };
const choices = (grants = [wide], served: GrantChoices['model'] = model): GrantChoices => ({ grants, model: served, problem: null });

describe('Access an agent may be given', () => {
  it('offers each passable action alone through only.<action>, never editor and never a withheld action', () => {
    const options = grantOptions(choices(), ADA, ADA, 'Ada');
    expect(options.map((option) => [option.relation, option.actions, option.reason])).toEqual([['only.edit', ['edit'], ''], ['only.view', ['view'], '']]);
  });
  it('offers nothing when Lys does not say which actions no agent may hold', () => {
    const unsaid = Object.fromEntries(Object.entries(model).filter(([key]) => key !== 'withheld_from_agents')) as unknown as GrantChoices['model'];
    expect(grantOptions(choices([wide], unsaid), ADA, ADA, 'Ada')).toEqual([]);
  });
  it('names why an app kind cannot be given, and never sends it', () => {
    const options = grantOptions(choices([{ ...wide, resource: { kind: 'fixture.doc', id: 'one' } }]), ADA, ADA, 'Ada');
    expect(options.length).toBeGreaterThan(0);
    expect(options.every((option) => option.reason === appAgentRefusal)).toBe(true);
    expect(() => newAgentGrants(options, options.map(grantOptionKey))).toThrow('can no longer be passed on');
  });
  it('names an app kind Lys has not declared beside its grant, without throwing, and never sends it', () => {
    const options = grantOptions(choices([{ ...wide, resource: { kind: 'notes.page', id: 'one' } }]), ADA, ADA, 'Ada');
    expect(options.length).toBeGreaterThan(0);
    expect(options.every((option) => option.reason === 'Lys has not declared the app kind notes.page; an administrator approves the app under Apps.')).toBe(true);
    expect(() => newAgentGrants(options, options.map(grantOptionKey))).toThrow('can no longer be passed on');
  });
  it('requires the app kinds when reading choices, and says so in a sentence instead of throwing while the form renders', async () => {
    const missing = Object.fromEntries(Object.entries(model).filter(([key]) => key !== 'kinds'));
    serve({ '/grants': ok({ grants: [wide], revision: 7 }), '/grants/model': ok(missing) });
    const read = await readGrantChoices();
    expect(read.model).toBeNull();
    expect(read.problem).toMatchObject({ refusal: { refusal: 'GrantModelUnreadable',
      reason: 'Lys did not say which app actions an agent may hold (the service did not answer the app kinds), so no access is offered. Reload the page to ask Lys again.' } });
    expect(grantOptions(read, ADA, ADA, 'Ada')).toEqual([]);
  });
  it('makes one grant per chosen action, each with its own operation', () => {
    const options = grantOptions(choices(), ADA, ADA, 'Ada');
    const given = newAgentGrants(options, options.map(grantOptionKey));
    expect(given.map((entry) => [entry.source, entry.relation, entry.actions])).toEqual([[wide.id, 'only.edit', ['edit']], [wide.id, 'only.view', ['view']]]);
    expect(new Set(given.map((entry) => entry.operation)).size).toBe(2);
  });
  it('refuses a choice it did not offer, sending nothing', () => {
    const options = grantOptions(choices(), ADA, ADA, 'Ada');
    expect(() => newAgentGrants(options, [wide.id + ':only.grant'])).toThrow('can no longer be passed on');
  });
});
