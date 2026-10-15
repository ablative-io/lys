/** The organisation's context default on an agent's Limits: said in plain words, given to the agent as its own on request, and changed for every agent only by an administrator. */
import { describe, expect, it } from 'vitest';
import { $, click, mount, text } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { BudgetBody, ContextDefault, Limit } from '../src/features/usage/contract';
import { budgetsView } from './budget-fixtures';
import { defaultWords } from '../src/features/usage/ContextDefault';

const budgets = '/budgets/agent/' + SCRIBE;
const organisation = '/budgets/organisation/organisation';
const file = '#/file/' + SCRIBE + '/budgets';
const level = (amount: number, act: Limit['act']): Limit => ({ unit: 'context_percent', amount, period: null, act });
const LEVELS = [level(20, 'notice'), level(38, 'notice'), level(40, 'compact')];
const inherited: ContextDefault = { limits: LEVELS, version: 0, built_in: true, set_aside: false };
const SENTENCE = 'Warns at 20% and 38%, compacts at 40% (the default for every agent).';

function service(admin: boolean, puts: BudgetBody[] = []): Record<string, Route> {
  const agent = { kind: 'agent' as const, id: SCRIBE };
  return {
    ...SERVICE,
    [budgets]: ok(budgetsView(agent, [], { context_default: inherited })),
    ['PUT ' + budgets]: (body) => {
      const given = body as BudgetBody;
      puts.push(given);
      return ok(budgetsView(agent, [], { limits: given.limits, version: given.version + 1, by: SCRIBE, at: 1,
        used: given.limits.map((limit) => ({ unit: limit.unit, period: limit.period, figure: 0, since_ms: null, unavailable: null })),
        context_default: { ...inherited, set_aside: true } }));
    },
    ['/agents/' + SCRIBE + '/usage']: ok({ agent: SCRIBE, used: [], receipts: [], last_reported_ms: 1 }),
    ['/agents/' + SCRIBE + '/goals']: ok({ goals: [] }),
    [organisation]: admin
      ? ok(budgetsView({ kind: 'organisation', id: 'organisation' }, [], { limits: LEVELS,
        used: LEVELS.map((limit) => ({ unit: limit.unit, period: null, figure: null, since_ms: null, unavailable: 'context_percent has not been reported' })) }))
      : refused(403, 'not_permitted', 'only an administrator reads or sets budgets on organisation organisation'),
  };
}

describe('the context default every agent inherits', () => {
  it('says the default in plain words, offers Change for this agent and hides Change the default from a non-administrator', async () => {
    await mount(file, service(false));
    expect(text()).toContain(SENTENCE);
    expect($('button[aria-label="Change for this agent"]')).not.toBeNull();
    expect($('button[aria-label="Change the default"]')).toBeNull();
    expect(text()).not.toContain('Whether you may change the default could not be read');
  });

  it('gives the agent the default levels as its own when Change for this agent is pressed', async () => {
    const puts: BudgetBody[] = [];
    await mount(file, service(false, puts));
    await click($('button[aria-label="Change for this agent"]'));
    expect(puts).toEqual([{ limits: LEVELS, warn_at: null, version: 0 }]);
  });

  it('opens the default for every agent to an administrator', async () => {
    await mount(file, service(true));
    await click($('button[aria-label="Change the default"]'));
    const tables = document.querySelectorAll('section[aria-label="Budgets"]');
    expect(tables).toHaveLength(2);
    const amounts = Array.from(tables[1].querySelectorAll<HTMLInputElement>('tbody input[type="number"]')).map((input) => input.value);
    expect(amounts).toEqual(['20', '38', '40']);
  });

  it('says when the agent’s own levels replace the default, and when the default is switched off', () => {
    expect(defaultWords(inherited)).toBe(SENTENCE);
    expect(defaultWords({ ...inherited, set_aside: true })).toBe('This agent’s own context levels replace the default for every agent (warns at 20% and 38%, compacts at 40%).');
    expect(defaultWords({ ...inherited, limits: [] })).toBe('No context default is in force: an administrator switched it off, so this agent is not warned or compacted unless it has levels of its own.');
  });
});
