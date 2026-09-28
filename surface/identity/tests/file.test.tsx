import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press, text, unreachable } from './harness';
import { unmountAll } from './harness';
import { ADA, REVIEWER, SCRIBE, SERVICE, refused } from './fixtures';

describe("An agent's file", () => {
  it('shows its person, state and registration from the service', async () => {
    const { requests } = await mount('#/file/' + SCRIBE);
    expect(requests).toContain('/directory/agents/' + SCRIBE);
    expect($('.file h1')?.textContent).toBe("Scribe");
    expect($('.file')?.dataset.tab).toBe('agent file · A/00000000');
    expect($('#state')?.textContent).toBe('active');
    expect($('.file .head .pill.human')?.getAttribute('href')).toBe('#/file/' + ADA);
    expect($('.file .head')?.textContent).toContain('since 22 Sep');
    expect($$('.tabs a').map((a) => a.textContent)).toEqual(['Role', 'Access1', 'Provisioning', 'Memory and context', 'Credentials', 'Sessions', 'Certificate', 'Record2']);
  });

  it('puts Start… after the lifecycle acts, just before Emergency stop', async () => {
    await mount('#/file/' + SCRIBE);
    const acts = $$('.file .head button').map((b) => b.dataset.act);
    expect(acts.slice(-2)).toEqual(['start', 'stop']);
    expect(acts.indexOf('suspend')).toBeLessThan(acts.indexOf('start'));
  });

  it('shows role and version as not recorded, never a sample role', async () => {
    await mount('#/file/' + SCRIBE);
    const facts = $('.facts')?.textContent ?? '';
    expect(text()).toContain('No role is currently assigned');
    expect(facts).toContain('Ada (test person)');
    expect(text()).not.toContain('Identity lead');
  });

  it('flags an agent whose person is retired', async () => {
    await mount('#/file/' + REVIEWER);
    expect($('.file .head')?.textContent).toContain('retired: needs a new person');
  });

  it('switches sections with keys 1 to 7, each its own address', async () => {
    await mount('#/file/' + SCRIBE);
    await press('2', {}, document.body);
    expect(location.hash).toBe(`#/file/${SCRIBE}/access`);
    expect($('.tabs a.on')?.textContent).toBe('Access1');
    await press('7', {}, document.body);
    expect(location.hash).toBe(`#/file/${SCRIBE}/certificate`);
    await press('1', {}, document.body);
    expect($('.tabs a.on')?.textContent).toBe('Role');
  });

  it('draws the record from the signed receipts', async () => {
    const { requests } = await mount(`#/file/${SCRIBE}/record`);
    expect(requests).toEqual(expect.arrayContaining(['/receipts/4', '/receipts/5']));
    const steps = $$('.timeline .tl');
    expect(steps.map((s) => s.querySelector('.when')?.textContent)).toEqual(['22 Sep 09:14', '22 Sep 09:20']);
    expect(steps[0].textContent).toContain('registered, under its person');
    expect(steps[1].classList.contains('now')).toBe(true);
    expect($$('.card a.mono').map((a) => a.getAttribute('href'))).toEqual(['/api/receipts/4', '/api/receipts/5']);
  });

  it('opens the lifecycle form without changing identity state before submission', async () => {
    const { posted } = await mount('#/file/' + SCRIBE);
    await click($('[data-act="suspend"]'));
    expect(location.hash).toBe('#/directory/manage?action=status&identity=' + SCRIBE);
    expect(document.querySelector<HTMLSelectElement>('select[name="identity"]')?.value).toBe(SCRIBE);
    expect($('form[aria-label="Record lifecycle change"]')).not.toBeNull();
    expect(posted).toHaveLength(0);
  });

  it('answers an agent that is not visible as not found, with the refusal', async () => {
    await mount('#/file/agent-' + 'f'.repeat(32), { ...SERVICE, ['/directory/agents/agent-' + 'f'.repeat(32)]: refused(404, 'AgentNotVisible', 'AgentNotVisible: no such agent is yours to see') });
    expect($('.page h1')?.textContent).toBe('Not found');
    expect($('.why-not b')?.textContent).toBe('AgentNotVisible');
  });
});

describe("A person's file", () => {
  it('lists the agents answering to them', async () => {
    await mount('#/file/' + ADA);
    expect($('.file')?.dataset.tab).toBe('person file · P/00000000');
    const rows = $$('.card .row a').map((a) => a.textContent);
    expect(rows).toEqual(["Scribe", "Courier", "Archivist"]);
    expect(unreachable()).toEqual([]);
  });
});
