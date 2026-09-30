import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press, text, unreachable, unmountAll } from './harness';
import { ADA, COURIER, REVIEWER, SCRIBE, SCRIBE_VIEW, SERVICE, ok, refused } from './fixtures';

describe("An agent's file", () => {
  it('shows its person, state and registration from the service', async () => {
    const { requests } = await mount('#/file/' + SCRIBE);
    expect(requests).toContain('/directory/agents/' + SCRIBE);
    expect($('.file h1')?.textContent).toBe("Scribe");
    expect($('.file')?.dataset.tab).toBe('agent file · A/00000000');
    expect($('#state')?.textContent).toBe('Active');
    expect($('[aria-label="About this agent"] .pill.human')?.getAttribute('href')).toBe('#/file/' + ADA);
    expect($('.agent-details')?.textContent).toContain('since 22 Sep');
    expect($$('.tabs a').map((a) => a.textContent)).toEqual(['Role', 'Access1', 'Start', 'Memory and context', 'Credentials', 'Sessions', 'Certificate', 'Tool policy', 'Record2', 'Budgets and goals']);
  });

  it('explains three next steps, keeps lifecycle controls in Details and preserves Emergency stop', async () => {
    await mount('#/file/' + SCRIBE);
    const next = $$('nav[aria-label="Next steps"] a');
    expect(next.map((entry) => entry.querySelector('strong')?.textContent)).toEqual(['Start', 'Set limits', 'Give access']);
    expect(next.map((entry) => entry.getAttribute('href'))).toEqual(['provisioning', 'budgets', 'access'].map((tab) => '#/file/' + SCRIBE + '/' + tab));
    expect(next.every((entry) => Boolean(entry.querySelector('span')?.textContent))).toBe(true);
    expect($('.agent-next a[data-act="start"]')?.getAttribute('href')).toBe('#/file/' + SCRIBE + '/provisioning');
    expect($('.agent-details [data-act="suspend"]')?.getAttribute('href')).toBe('#/directory/manage?action=status&identity=' + SCRIBE);
    expect($('.file .head button[data-act="stop"]')).not.toBeNull();
    expect($('.agent-details [data-act="stop"]')).toBeNull();
  });

  it('stops an agent only after a reason is given, under one operation, and never claims a session ended', async () => {
    const { posted } = await mount('#/file/' + SCRIBE, { ...SERVICE, ['POST /agents/' + SCRIBE + '/stop']: (body) => ok({ agent: SCRIBE, operation: (body as { operation: string }).operation, state: 'suspended', by: ADA, at: 1790000200, certificates_withdrawn: ['op-' + 'a'.repeat(32)], credentials_ended: null, credentials_refused: 'SecretsUnavailable: no secrets broker is configured', sessions_asked: ['op-' + 'b'.repeat(32)], reason: 'leaked its key' }) });
    await click($('.file .head button[data-act="stop"]'));
    expect(posted).toEqual([]);
    const reason = $('form[aria-label="Confirm emergency stop"] input');
    if (!(reason instanceof HTMLInputElement)) throw new Error('Reason field missing');
    await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(reason, 'leaked its key'); reason.dispatchEvent(new Event('input', { bubbles: true })); });
    await click([...document.querySelectorAll('button')].find((entry) => entry.textContent === 'Stop this agent now') ?? null);
    expect(posted).toEqual([{ path: '/agents/' + SCRIBE + '/stop', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), reason: 'leaked its key' } }]);
    expect(text()).toContain('The agent’s access is suspended.');
    expect(text()).toContain('The credential service did not confirm that credentials ended.');
    expect(text()).toContain('Other sessions remain unconfirmed');
    expect($('[aria-label="Emergency stop recorded"]')).not.toBeNull();
  });

  it('recovers an unanswered stop after the next read shows the agent suspended', async () => {
    sessionStorage.clear();
    const path = '/agents/' + SCRIBE + '/stop';
    const first = await mount('#/file/' + SCRIBE, { ...SERVICE, ['POST ' + path]: refused(503, 'StopsUnavailable', 'Outcome unknown') });
    await click($('[data-act="stop"]'));
    const reason = $('form[aria-label="Confirm emergency stop"] input');
    if (!(reason instanceof HTMLInputElement)) throw new Error('Reason field missing');
    await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(reason, 'Key exposed'); reason.dispatchEvent(new Event('input', { bubbles: true })); });
    await click([...document.querySelectorAll('button')].find((entry) => entry.textContent === 'Stop this agent now') ?? null);
    expect(first.posted).toHaveLength(1);
    unmountAll(); document.body.innerHTML = '';
    const later = await mount('#/file/' + SCRIBE, { ...SERVICE, ['/directory/agents/' + SCRIBE]: ok({ ...SCRIBE_VIEW, state: 'suspended' }), ['POST ' + path]: (body) => ok({ agent: SCRIBE, operation: (body as { operation: string }).operation, state: 'suspended', by: ADA, at: 1790000200, certificates_withdrawn: [], credentials_ended: null, credentials_refused: 'Broker unavailable', sessions_asked: [], reason: 'Key exposed' }) });
    expect($('[data-act="stop"]')).toBeNull();
    await click([...document.querySelectorAll('button')].find((entry) => entry.textContent === 'Check whether Lys saved it') ?? null);
    expect(later.posted).toEqual(first.posted);
    expect(text()).toContain('Broker unavailable');
    expect($('[aria-label="Emergency stop recorded"]')).not.toBeNull();
    expect(sessionStorage.getItem('lys.pending.stop.' + SCRIBE)).toBeNull();
  });

  it('shows role and version as not recorded, never a sample role', async () => {
    await mount('#/file/' + SCRIBE);
    const details = $('.agent-details');
    if (!(details instanceof HTMLDetailsElement)) throw new Error('Agent details missing');
    await act(async () => { details.open = true; details.dispatchEvent(new Event('toggle')); });
    const facts = $('.facts')?.textContent ?? '';
    expect(text()).toContain('No role is currently assigned');
    expect(facts).toContain('Ada (test person)');
    expect(text()).not.toContain('Identity lead');
  });

  it('flags an agent whose person is retired', async () => {
    await mount('#/file/' + REVIEWER);
    expect($('[aria-label="About this agent"]')?.textContent).toContain('retired: needs a new person');
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
    const details = $('.agent-details');
    if (!(details instanceof HTMLDetailsElement)) throw new Error('Agent details missing');
    await act(async () => { details.open = true; details.dispatchEvent(new Event('toggle')); });
    await click($('[data-act="suspend"]'));
    expect(location.hash).toBe('#/directory/manage?action=status&identity=' + SCRIBE);
    expect(document.querySelector<HTMLInputElement>('input[type="hidden"][name="identity"]')?.value).toBe(SCRIBE);
    expect($('form[aria-label="Record lifecycle change"]')).not.toBeNull();
    expect(posted).toHaveLength(0);
  });

  it('offers a registered identity only activation, since the directory retires only an active or suspended one', async () => {
    await mount('#/directory/manage?action=status&identity=' + COURIER);
    expect($$('select[name="transition"] option').map((option) => option.getAttribute('value')).filter(Boolean)).toEqual(['activate']);
  });

  it('answers an agent that is not visible as not found, with the refusal', async () => {
    await mount('#/file/agent-' + 'f'.repeat(32), { ...SERVICE, ['/directory/agents/agent-' + 'f'.repeat(32)]: refused(404, 'AgentNotVisible', 'AgentNotVisible: no such agent is yours to see') });
    expect($('.page h1')?.textContent).toBe('Not found');
    expect($('.why-not details')?.textContent).toContain('AgentNotVisible');
    expect($('.why-not p')?.textContent).toContain('Ask its responsible person');
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
