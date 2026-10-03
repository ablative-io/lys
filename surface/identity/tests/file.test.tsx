import { act } from 'react';
import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press, text, unreachable, unmountAll } from './harness';
import { ADA, COURIER, REVIEWER, SCRIBE, SCRIBE_VIEW, SERVICE, ok, refused } from './fixtures';

/** What an agent's Overview reads for its run: its settings, the computers and the offered choices. */
const RUN = {
  ['/agents/' + SCRIBE + '/provisioning']: ok({ agent: SCRIBE, profile: null, versions: [], enforced: false }),
  '/network': ok({ machines: [], reports_served: true }), '/harnesses': ok({ programs: [] }), '/skills': ok({ skills: [] }), '/secrets': ok({ secrets: [] }),
};

describe("An agent's file", () => {
  it('shows its person, state and registration from the service', async () => {
    const { requests } = await mount('#/file/' + SCRIBE);
    expect(requests).toContain('/directory/agents/' + SCRIBE);
    expect($('.file h1')?.textContent).toBe("Scribe");
    expect($('.file')?.dataset.tab).toBe('agent file · A/00000000');
    expect($('#state')?.textContent).toBe('Active');
    expect($('[aria-label="About this agent"] .pill.human')?.getAttribute('href')).toBe('#/file/' + ADA);
    expect($('.file .head')?.textContent).toContain('since 22 Sep');
    expect($('.agent-details')).toBeNull();
    expect($('.file details')).toBeNull();
    expect($$('.tabs a').map((a) => a.textContent)).toEqual(['Overview', 'Settings', 'Access1', 'Limits and goals', 'Sessions', 'Credentials', 'Record2']);
  });

  it('starts from its own Overview, keeps lifecycle controls in the head, in view, and preserves Emergency stop', async () => {
    await mount('#/file/' + SCRIBE, { ...SERVICE, ...RUN });
    expect($('nav[aria-label="Next steps"]')).toBeNull();
    expect($('section[aria-label="Run"]')?.textContent).toContain('Scribe is not running.');
    expect($('a[href^="#/team"]')).toBeNull();
    expect($('.file .head [data-act="suspend"]')).not.toBeNull();
    expect($('.file .head button[data-act="stop"]')).not.toBeNull();
    expect($$('.file [data-act="stop"]').length).toBe(1);
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
    const about = $('[aria-label="About this agent"]')?.textContent ?? '';
    expect(text()).toContain('No role is currently assigned');
    expect(about).toContain('Ada (test person)');
    expect(text()).not.toContain('Identity lead');
  });

  it('flags an agent whose person is retired', async () => {
    await mount('#/file/' + REVIEWER);
    expect($('[aria-label="About this agent"]')?.textContent).toContain('retired: needs a new person');
  });

  it('switches sections with keys 1 to 7, each its own address', async () => {
    await mount('#/file/' + SCRIBE);
    await press('3', {}, document.body);
    expect(location.hash).toBe(`#/file/${SCRIBE}/access`);
    expect($('.tabs a.on')?.textContent).toBe('Access1');
    await press('7', {}, document.body);
    expect(location.hash).toBe(`#/file/${SCRIBE}/record`);
    expect($('.tabs a.on')?.textContent).toBe('Record2');
    await press('1', {}, document.body);
    expect($('.tabs a.on')?.textContent).toBe('Overview');
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

  it('opens the lifecycle form in the head of the file itself, without leaving the page or changing identity state before submission', async () => {
    const { posted } = await mount('#/file/' + SCRIBE);
    await click($('.file .head [data-act="suspend"]'));
    expect(location.hash).toBe('#/file/' + SCRIBE);
    const form = $('.file form[aria-label="Record lifecycle change"]');
    expect(form).not.toBeNull();
    expect(form?.querySelector('input[name="reason"]')).not.toBeNull();
    expect(form?.querySelector('[name="identity"]')).toBeNull();
    expect($('#state')?.textContent).toBe('Active');
    expect(posted).toHaveLength(0);
  });

  it('edits the name in the head of the file itself', async () => {
    const { posted } = await mount('#/file/' + SCRIBE);
    await click($('.file .head [data-act="rename"]'));
    expect(location.hash).toBe('#/file/' + SCRIBE);
    expect(document.querySelector<HTMLInputElement>('.file form[aria-label="Save name"] input[name="display_name"]')?.value).toBe('Scribe');
    expect(posted).toHaveLength(0);
  });

  it('offers a registered identity only activation, since the directory retires only an active or suspended one', async () => {
    await mount('#/file/' + COURIER, { ...SERVICE, ...RUN, ['/directory/agents/' + COURIER]: ok({ ...SCRIBE_VIEW, id: COURIER, display_name: 'Courier', state: 'registered' }), ['/agents/' + COURIER + '/provisioning']: ok({ agent: COURIER, profile: null, versions: [], enforced: false }), ['/agents/' + COURIER + '/runtime/sessions']: ok({ sessions: [] }) });
    expect($$('.file .head button[data-act]').map((button) => button.getAttribute('data-act')).filter((act) => ['activate', 'suspend', 'retire', 'reinstate'].includes(act ?? ''))).toEqual(['activate']);
  });

  it('answers an agent that is not visible as not found, with the refusal', async () => {
    await mount('#/file/agent-' + 'f'.repeat(32), { ...SERVICE, ['/directory/agents/agent-' + 'f'.repeat(32)]: refused(404, 'AgentNotVisible', 'AgentNotVisible: no such agent is yours to see') });
    expect($('.page h1')?.textContent).toBe('Not found');
    expect($('.why-not .refusal-name')?.textContent).toContain('AgentNotVisible');
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
