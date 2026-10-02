import { describe, expect, it } from 'vitest';
import { actionWords, resourceWords } from '../src/features/grants/action-words';
import { readGrantWorld, nameOf } from '../src/features/grants/model';
import type { RuntimeSession } from '../src/features/runtime/RuntimeSessions';
import { graphFromRecords } from '../src/features/runtime/session-graph';
import { ADA, GRANTS, ME, MODEL, SERVICE, ok } from './fixtures';
import { serve } from './harness';

describe('Served action sentences', () => {
  const model = { ...MODEL, action_sentences: { 'agent.start': 'Start this agent', read: 'Read this resource' } };
  it('uses served words and collapses exactly the complete shipped set', () => {
    const resource = { kind: 'agent', id: 'one' };
    expect(actionWords(model, resource, ['agent.start'])).toBe('Start this agent');
    expect(actionWords(model, resource, ['read', 'agent.start'])).toBe('everything here');
    expect(actionWords(model, resource, [])).toBe('');
  });
  it('preserves app action names even when they match shipped names or the complete set', () => {
    expect(actionWords(model, { kind: 'notes.page', id: 'one' }, ['read', 'agent.start'])).toBe('read; agent.start');
  });
  it('names the setup service and the apps directory from the records', async () => {
    serve({ ...SERVICE, '/me': ok({ ...ME, service_accounts: [{ id: 'service-account-one', owner: ADA, name: 'Lys directory loader', state: 'active' }] }) });
    const world = await readGrantWorld();
    expect(nameOf(world, 'service-account-one')).toBe("Lys's own service");
    expect(resourceWords({ kind: 'directory', id: 'apps' })).toBe('the apps directory');
  });
  it('uses the same served words on runtime grant edges', async () => {
    serve(SERVICE);
    const world = await readGrantWorld();
    const grant = { ...GRANTS[0], holder: 'agent-one', actions: ['agent.start'], resource: { kind: 'agent', id: 'one' } };
    const graph = graphFromRecords([{ session: 'session-one', agent: 'agent-one', machine: 'machine-one' } as RuntimeSession], [], { ...world, model, list: { ...world.list, grants: [grant] } });
    expect(graph.edges[0].label).toBe('Start this agent on agent one');
  });
});
