import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press, text, unreachable } from './harness';
import { ADA, DIRECTORY, SCRIBE, SERVICE, ok, refused } from './fixtures';

const names = () => $$('tbody tr[data-pick] td:first-child').map((td) => td.textContent);

describe('People and agents', () => {
  it('lists the directory from the service: each person, then the agents that answer to them', async () => {
    const { requests } = await mount('#/people');
    expect(requests).toContain('/directory/people');
    expect(names()).toEqual(['Ada (test person)', "Scribe", "Courier", "Archivist", 'Bea (test person)', "Reviewer", "Lamplighter"]);
    const scribe = $(`tr[data-href="#/file/${SCRIBE}"]`);
    expect(scribe?.textContent).toContain('active');
    expect(scribe?.textContent).toContain('Ada (test person)');
    expect($$('.stat .n').map((n) => n.textContent)).toEqual(['1', '2', '1', '0', '0']);
  });

  it('flags an agent whose person is retired (conformance 3.1)', async () => {
    await mount('#/people');
    const reviewer = $$('tbody tr').find((tr) => tr.textContent?.includes("Reviewer"));
    expect(reviewer?.textContent).toContain('(retired)');
    const lamplighter = $$('tbody tr').find((tr) => tr.textContent?.includes("Lamplighter"));
    expect(lamplighter?.textContent).not.toContain('(retired)');
  });

  it('flags an agent whose person is suspended, not only retired (conformance 3.1)', async () => {
    const suspended = { ...DIRECTORY, people: DIRECTORY.people.map((p) => (p.id === ADA ? { ...p, state: 'suspended' as const } : p)) };
    await mount('#/people', { ...SERVICE, '/directory/people': ok(suspended) });
    const scribe = $(`tr[data-href="#/file/${SCRIBE}"]`);
    expect(scribe?.textContent).toContain('(suspended)');
    expect($$('.stat .n')[2].textContent).toBe('4');
  });

  it('shows no sample data and reads reported counts', async () => {
    await mount('#/people', { ...SERVICE, '/runtime/sessions': ok({ sessions: [{ agent: null, shown: 'unconfirmed' }, { agent: SCRIBE, shown: 'running' }] }) });
    for (const sample of ['Tom Whiting', 'Dana Reyes', 'Night builder', 'mock-up · sample data']) expect(text()).not.toContain(sample);
    expect($$('.stat .n').slice(3).map((entry) => entry.textContent)).toEqual(['1', '1']);
    expect(text()).not.toContain('not built yet');
  });

  it('moves with j and k, previews the row, and opens it with Enter', async () => {
    await mount('#/people');
    expect($('.detail h2')?.textContent).toBe('Ada (test person)');
    await press('j', {}, document.body);
    await press('j', {}, document.body);
    expect($('tr.cursor td')?.textContent).toBe("Courier");
    expect($('.detail h2')?.textContent).toBe("Courier");
    await press('k', {}, document.body);
    expect($('.detail h2')?.textContent).toBe('Scribe');
    await press('Enter', {}, document.body);
    expect(location.hash).toBe('#/file/' + SCRIBE);
  });

  it('opens a focused row with Enter or Space, and every control is reachable (9.3)', async () => {
    await mount('#/people');
    expect(unreachable()).toEqual([]);
    const row = $(`tr[data-href="#/file/${SCRIBE}"]`);
    expect(row?.tabIndex).toBe(0);
    row?.focus();
    await press(' ');
    expect(location.hash).toBe('#/file/' + SCRIBE);
  });

  it('filters people and agents, and reads the teams registry', async () => {
    await mount('#/people', { ...SERVICE, '/teams': ok({ teams: [] }) });
    await click($('[data-kind="agent"]'));
    expect(names()).toHaveLength(5);
    await click($('[data-kind="person"]'));
    expect(names()).toEqual(['Ada (test person)', 'Bea (test person)']);
    await click($('[data-kind="teams"]'));
    expect(text()).toContain('No teams yet.');
  });

  it('falls back to the personal view when not admitted to the directory', async () => {
    const { requests } = await mount('#/people', {
      ...SERVICE,
      '/directory/people': refused(403, 'NotAdmitted', 'NotAdmitted: only the configured administrator may do this in step 1'),
    });
    expect(requests).toEqual(expect.arrayContaining(['/directory/people', '/people']));
    expect(names()).toEqual(['Ada (test person)', "Scribe", "Courier", "Archivist"]);
    expect(text()).toContain('Your own records');
  });

  it('asks a caller with no session to sign in through the service', async () => {
    await mount('#/people', { ...SERVICE, '/directory/people': refused(401, 'NotSignedIn', 'NotSignedIn: sign in through the configured issuer first') });
    expect($('.page h1')?.textContent).toBe('Sign in');
    expect($('a.btn.primary')?.getAttribute('href')).toBe('/api/login');
  });

  it('names a refusal the service gives', async () => {
    await mount('#/people', {
      ...SERVICE,
      '/directory/people': refused(403, 'NotAdmitted', 'NotAdmitted: x'),
      '/people': refused(403, 'NoPerson', 'NoPerson: your login is bound to no person'),
    });
    expect($('.why-not p')?.textContent).toContain('Ask the administrator to connect it');
    expect($('.why-not details')?.textContent).toContain('NoPerson');
    expect($('.why-not details')?.hasAttribute('open')).toBe(false);
    expect(text()).toContain('your login is bound to no person');
    expect(ADA).toMatch(/^person-/);
  });
  it('names a refused runtime read instead of reporting zero', async () => {
    await mount('#/people', { ...SERVICE, '/runtime/sessions': refused(503, 'RuntimeUnavailable', 'No reports store') });
    expect($$('.stat .n').slice(3).map((entry) => entry.textContent)).toEqual(['—', '—']); expect(text()).toContain('RuntimeUnavailable');
  });
  it('takes the agent preview start action to the real profile screen', async () => {
    await mount('#/people'); await click($('[data-kind="agent"]'));
    expect($('[data-act="start"]')?.getAttribute('href')).toBe('#/file/' + SCRIBE + '/provisioning');
  });

});
