import { afterEach, describe, expect, it, vi } from 'vitest';
import { grantKindsProblem } from '../src/generated/grants';
import { agentMayHold, appAgentCarriers, singleActionCarriers } from '../src/features/grants/ActionPicker';
import { readGrantWorld, requireGrantKinds } from '../src/features/grants/model';
import { DIRECTORY, GRANTS, ME, MODEL, ok } from './fixtures';
import { serve } from './harness';

afterEach(() => vi.unstubAllGlobals());

describe('Required grant model kinds', () => {
  it('names a missing kinds member as an unreadable model before interpreting agent permission', async () => {
    const missing = { ...MODEL };
    Reflect.deleteProperty(missing, 'kinds');
    expect(grantKindsProblem(missing)).toBe('the service did not answer the app kinds');
    serve({ '/directory/people': ok(DIRECTORY), '/grants': ok({ grants: GRANTS, revision: 7 }), '/grants/model': ok(missing) });
    await expect(readGrantWorld(ME)).rejects.toMatchObject({
      refusal: { refusal: 'GrantModelUnreadable', reason: 'the service did not answer the app kinds' },
    });
    for (const read of [
      () => requireGrantKinds(missing),
      () => agentMayHold(missing, 'cambium.channel', ['read']),
      () => appAgentCarriers(missing, 'cambium.channel', ['read']),
      () => singleActionCarriers(missing, 'cambium.channel'),
    ]) expect(read).toThrow('the service did not answer the app kinds');
  });

  it.each([null, [], 'unreadable', 6])('refuses a non-object kinds member %j without manufacturing a declaration', (kinds) => {
    expect(grantKindsProblem({ ...MODEL, kinds })).toBe('the service did not answer the app kinds');
  });

  it('names malformed agent actions and relation carriers separately', () => {
    expect(grantKindsProblem({ ...MODEL, kinds: { 'cambium.channel': { agents: [6], relations: {} } } }))
      .toBe('the service did not answer valid agent actions for cambium.channel');
    expect(grantKindsProblem({ ...MODEL, kinds: { 'cambium.channel': { agents: ['read'], relations: { reader: 'read' } } } }))
      .toBe('the service did not answer valid relations for cambium.channel');
  });

  it('reads an explicit empty kinds object and an explicit app refusal as different answers', () => {
    expect(grantKindsProblem({ ...MODEL, kinds: {} })).toBeNull();
    expect(agentMayHold(MODEL, 'fixture.doc', ['read'])).toBe(false);
    expect(appAgentCarriers(MODEL, 'fixture.doc', ['read'])).toEqual([]);
    expect(() => agentMayHold({ ...MODEL, kinds: {} }, 'fixture.doc', ['read']))
      .toThrow('the service did not answer the app kind fixture.doc');
  });
});
