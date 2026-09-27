import { describe, expect, it } from 'vitest';
import { $, $$, click, mount, press, text, unreachable } from './harness';
import { ADA, SCRIBE, SERVICE, refused } from './fixtures';

const names = () => $$('tbody tr[data-pick] td:first-child').map((td) => td.textContent);

describe('People and agents', () => {
  it('lists the directory from the service: people first, then agents with their person', async () => {
    const { requests } = await mount('#/people');
    expect(requests).toContain('/directory/people');
    expect(names()).toEqual(['Ada (test person)', 'Bea (test person)', "Scribe", "Courier", "Archivist", "Reviewer", "Lamplighter"]);
    const scribe = $(`tr[data-href="#/file/${SCRIBE}"]`);
    expect(scribe?.textContent).toContain('agent');
    expect(scribe?.textContent).toContain('active');
    expect(scribe?.textContent).toContain('Ada (test person)');
    expect($$('.stat .n').map((n) => n.textContent)).toEqual(['1', '2', '1', '—', '—']);
  });

  it('flags an agent whose person is retired (conformance 3.1)', async () => {
    await mount('#/people');
    const reviewer = $$('tbody tr').find((tr) => tr.textContent?.includes("Reviewer"));
    expect(reviewer?.textContent).toContain('(retired)');
    const lamplighter = $$('tbody tr').find((tr) => tr.textContent?.includes("Lamplighter"));
    expect(lamplighter?.textContent).not.toContain('(retired)');
  });

  it('shows no sample data and marks what has no server', async () => {
    await mount('#/people');
    for (const sample of ['Tom Whiting', 'Dana Reyes', 'Night builder', 'mock-up · sample data']) expect(text()).not.toContain(sample);
    expect(text()).toContain('not built yet');
  });

  it('moves with j and k, previews the row, and opens it with Enter', async () => {
    await mount('#/people');
    expect($('.preview h2')?.textContent).toBe('Ada (test person)');
    await press('j', {}, document.body);
    await press('j', {}, document.body);
    expect($('tr.cursor td')?.textContent).toBe("Scribe");
    expect($('.preview h2')?.textContent).toBe("Scribe");
    await press('k', {}, document.body);
    expect($('.preview h2')?.textContent).toBe('Bea (test person)');
    await press('Enter', {}, document.body);
    expect(location.hash).toMatch(/^#\/file\/person-/);
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

  it('filters people and agents, and says teams are not built', async () => {
    await mount('#/people');
    await click($('[data-kind="agent"]'));
    expect(names()).toHaveLength(5);
    await click($('[data-kind="person"]'));
    expect(names()).toEqual(['Ada (test person)', 'Bea (test person)']);
    await click($('[data-kind="teams"]'));
    expect($('.empty-note')?.textContent).toContain('not built yet');
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
    expect($('.why-not b')?.textContent).toBe('NoPerson');
    expect(text()).toContain('your login is bound to no person');
    expect(ADA).toMatch(/^person-/);
  });
});
