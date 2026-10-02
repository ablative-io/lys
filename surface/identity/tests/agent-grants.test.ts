import { describe, expect, it } from 'vitest';
import { grantOptions, newAgentGrants } from '../src/features/people/agent-grants';
import type { GrantChoices } from '../src/features/people/agent-grants';
import { ADA, GRANTS, MODEL } from './fixtures';

const source = GRANTS[0];
const model = { ...MODEL, relations: { ...MODEL.relations, 'only.edit': ['edit'], 'only.view': ['view'], 'only.grant': ['grant'] }, withheld_from_agents: ['grant'] };
const choices = (grants = GRANTS, served: GrantChoices['model'] = model): GrantChoices => ({ grants, model: served, problem: null });

describe('Access an agent may be given', () => {
  it('offers each passable action alone and never one withheld from agents or editor', () => {
    const wide = { ...source, pass_on: { kind: 'to' as const, actions: ['edit', 'grant', 'view'], recipients: ['agent' as const] } };
    const [option] = grantOptions(choices([wide]), ADA, ADA, 'Ada');
    expect(option).toMatchObject({ reason: '', actions: ['edit', 'view'] });
  });
  it('gives an app kind nothing', () => {
    const app = { ...source, resource: { kind: 'notes.page', id: 'one' } };
    expect(grantOptions(choices([app]), ADA, ADA, 'Ada')[0].reason).toBe('Nothing in this access can be given to an agent.');
  });
  it('makes one grant per chosen action, and everything here is every offered action', () => {
    const options = grantOptions(choices(), ADA, ADA, 'Ada');
    expect(newAgentGrants(options, { [source.id]: ['view'] }).map((entry) => [entry.relation, entry.actions])).toEqual([['only.view', ['view']]]);
    const all = newAgentGrants(options, [source.id]);
    expect(all.map((entry) => entry.relation)).toEqual(['only.edit', 'only.view']);
    expect(new Set(all.map((entry) => entry.operation)).size).toBe(2);
  });
  it('refuses an action the grant does not offer, sending nothing', () => {
    const options = grantOptions(choices(), ADA, ADA, 'Ada');
    expect(() => newAgentGrants(options, { [source.id]: ['grant'] })).toThrow('can no longer be passed on');
  });
});
