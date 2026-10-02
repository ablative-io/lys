import { describe, expect, it } from 'vitest';
import { agentGrantsOf, grantOptions, newAgentGrants } from '../src/features/people/agent-grants';
import { ADA, GRANTS, MODEL } from './fixtures';

const source = { ...GRANTS[0], actions: ['agent.start', 'agent.stop', 'grant.revoke', 'read'],
  pass_on: { kind: 'to' as const, actions: ['agent.start', 'agent.stop', 'grant.revoke', 'read'], recipients: ['agent' as const] } };
const model = { ...MODEL, withheld_from_agents: ['grant.revoke'],
  action_sentences: { 'agent.start': 'Start this agent', 'agent.stop': 'Stop this agent', read: 'Read this resource', 'grant.revoke': 'Withdraw this access' },
  relations: { owner: source.actions, editor: source.actions, 'only.agent.start': ['agent.start'], 'only.agent.stop': ['agent.stop'], 'only.grant.revoke': ['grant.revoke'], reader: ['read'] } };
const choices = { grants: [source], model, problem: null };

describe('Per action agent grants', () => {
  it('offers only single action carriers and excludes the served withheld acts', () => {
    const options = grantOptions(choices, ADA, ADA, 'Ada');
    expect(options.map((option) => option.relation).sort()).toEqual(['only.agent.start', 'only.agent.stop', 'reader']);
    expect(options.flatMap((option) => option.actions)).not.toContain('grant.revoke');
  });
  it('offers nothing to agents when the withheld declaration is missing', () => {
    expect(grantOptions({ ...choices, model: { ...MODEL } }, ADA, ADA, 'Ada')).toEqual([]);
  });
  it('keeps app actions visible but refuses an agent grant plan before sending', () => {
    const app = { ...source, resource: { kind: 'fixture.doc', id: 'doc7' }, relation: 'reader', actions: ['read'] };
    const options = grantOptions({ ...choices, grants: [app] }, ADA, ADA, 'Ada');
    expect(options).toHaveLength(1);
    expect(options[0].reason).toBe("Lys can't give an agent this app's actions until the app allows it.");
    expect(() => newAgentGrants(options, [app.id + ':reader'])).toThrow('GrantChoiceUnavailable');
  });
  it('records each selected act as its own operation and retains both from one source', () => {
    const options = grantOptions(choices, ADA, ADA, 'Ada');
    const plan = newAgentGrants(options, options.map((option) => source.id + ':' + option.relation));
    expect(plan).toHaveLength(3);
    expect(new Set(plan.map((entry) => entry.operation)).size).toBe(3);
    expect(plan.map((entry) => entry.actions)).toEqual(options.map((option) => option.actions));
    expect(agentGrantsOf(plan, null, null)).toEqual(plan);
  });
});
