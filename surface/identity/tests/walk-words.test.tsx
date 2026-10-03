import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { $, $$, choose, click, mount, unmountAll } from './harness';
import { ADA, GRANTS, ROOT_G, SCRIBE, SCRIBE_G, SERVICE, ok } from './fixtures';
import type { DelegateBody, GrantModel } from '../src/generated/grants';

const resource = { kind: 'directory', id: 'agents' };
const model: GrantModel = {
  version: 1,
  action_sentences: { 'agent.stop': 'Stop this agent', 'agent.start': 'Start this agent' },
  relations: { editor: ['agent.start', 'agent.stop'], 'only.agent.stop': ['agent.stop'], 'only.agent.start': ['agent.start'] },
  withheld_from_agents: [],
};
const grants = GRANTS.map((grant) => ({ ...grant, resource, relation: grant.id === ROOT_G ? 'editor' : 'only.agent.stop',
  actions: grant.id === ROOT_G ? ['agent.start', 'agent.stop'] : ['agent.stop'],
  pass_on: grant.id === ROOT_G ? { kind: 'to', actions: ['agent.start', 'agent.stop'], recipients: ['agent'] } : grant.pass_on }));
const routes = { ...SERVICE, '/grants/model': ok(model), '/grants': ok({ grants, revision: 7 }),
  [`/grants/cannot-give?route=browser&source=${ROOT_G}&recipient=${SCRIBE}`]: ok({ source: ROOT_G, recipient: SCRIBE, items: [] }),
  'POST /grants/reach': ok({ revision: 7, resources: [{ ...resource, holders: [{ holder: SCRIBE, actions: ['agent.stop'] }] }] }),
};

describe('Walk words', () => {
  it('explains recipient_kind_excluded without exposing a contract error', async () => {
    await mount('#/me?tab=account', { ...routes,
      [`/grants/cannot-give?route=browser&source=${ROOT_G}&recipient=${SCRIBE}`]: ok({ source: ROOT_G, recipient: SCRIBE,
        items: [{ subject: 'grant', grant: ROOT_G, reason: 'recipient_kind_excluded', source: true }] }),
    });
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect($('.act-panel [data-refusal]')).toBeNull();
    expect($('.act-panel [data-cannot-give="recipient_kind_excluded"] .note')?.textContent)
      .toBe('This access cannot be passed on to this kind of recipient.');
  });

  for (const hash of [`#/file/${SCRIBE}/access`, '#/access/can']) {
    it(`uses the model's action sentences in the check picker at ${hash}, retaining wire values`, async () => {
      const mounted = await mount(hash, routes);
      expect($$('#cPerm option').map((option) => option.textContent)).toEqual(['Start this agent', 'Stop this agent']);
      expect($$('#cPerm option').map((option) => (option as HTMLOptionElement).value)).toEqual(['agent.start', 'agent.stop']);
      expect($('#cRes option')?.textContent).toBe('the agents directory');
      await choose($('#cPerm'), 'agent.stop');
      await click($('[data-act="check"]'));
      expect(mounted.posted.filter((entry) => ['/grants/who', '/grants/why'].includes(entry.path)).at(-1)?.body)
        .toMatchObject({ resource, action: 'agent.stop' });
    });
  }

  it('names the source in the drawer and the confirmed grant in the toast', async () => {
    const mounted = await mount('#/me?tab=account', { ...routes,
      'POST /grants': (body: unknown) => ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, index: 1, receipt: { caller: ADA } }),
    });
    await click($(`[data-act="delegate"][data-g="${ROOT_G}"]`));
    expect($('.act-panel h2')?.textContent).toBe('Give part of editor of the agents directory to an agent');
    expect($$('.act-panel legend').map((legend) => legend.textContent)).toEqual(['Actions']);
    expect($$('.act-panel .field > label').map((label) => label.textContent)).not.toContain('Actions');
    await click($('.act-panel input[name="action"][value="agent.stop"]'));
    await click($('[data-act="delegatedo"]'));
    expect(mounted.posted.find((entry) => entry.path === '/grants')?.body)
      .toMatchObject({ source: ROOT_G, recipient: SCRIBE, relation: 'only.agent.stop' });
    expect($('#toast')?.textContent).toContain('Given. Scribe can now Stop this agent on the agents directory, through you.');
  });

  it('shows the served reach in plain words without claiming it is built in', async () => {
    await mount('#/access/reach/' + SCRIBE, routes);
    expect($$('.check .card tr').map((row) => row.textContent)).toEqual(['the agents directoryStop this agent']);
    expect($('.check .built-in')).toBeNull();
  });

  it('keeps request checkboxes beside their labels and each access heading once, as in the drawer', async () => {
    const style = document.createElement('style');
    style.textContent = ['forms', 'overlays'].map((name) => readFileSync(`src/styles/${name}.css`, 'utf8')).join('\n')
      + readFileSync('src/features/people/recorded-form.css', 'utf8');
    document.body.append(style);
    await mount('#/requests', { ...routes, '/requests': ok({ requests: [] }) });
    const form = $('form[aria-label="Ask for access"]');
    await choose(form?.querySelector('select') ?? null, '0');
    const checkboxes = [...(form?.querySelectorAll<HTMLInputElement>('input[name="action"], input[name="all_actions"]') ?? [])];
    expect(checkboxes).toHaveLength(3);
    expect([...form?.querySelectorAll('legend') ?? []].map((legend) => legend.textContent)).toEqual(['Access needed']);
    expect([...form?.querySelectorAll('.field > label') ?? []].map((label) => label.textContent)).not.toContain('Access needed');
    expect(checkboxes.map((input) => input.closest('tr')?.querySelector('label')?.textContent?.trim())).toEqual(['Everything here', 'Start this agent', 'Stop this agent']);
    for (const input of checkboxes) {
      const label = input.closest('label');
      expect(label).not.toBeNull();
      expect(getComputedStyle(label as HTMLLabelElement).display).toBe('flex');
      expect(getComputedStyle(input).width).toBe('15px');
      expect(getComputedStyle(input).padding).toBe('0px');
    }
    await click(checkboxes[0]);
    expect(checkboxes.every((input) => input.checked)).toBe(true);
    unmountAll();
  });
});
