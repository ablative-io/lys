import { describe, expect, it } from 'vitest';
import { grantOptionKey, grantOptions, newAgentGrants } from '../src/features/people/agent-grants';
import type { GrantChoices } from '../src/features/people/agent-grants';
import { appAgentRefusal } from '../src/features/grants/ActionPicker';
import { ADA, GRANTS, MODEL } from './fixtures';

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
    const options = grantOptions(choices([{ ...wide, resource: { kind: 'notes.page', id: 'one' } }]), ADA, ADA, 'Ada');
    expect(options.every((option) => option.reason === appAgentRefusal)).toBe(true);
    expect(() => newAgentGrants(options, options.map(grantOptionKey))).toThrow('can no longer be passed on');
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
