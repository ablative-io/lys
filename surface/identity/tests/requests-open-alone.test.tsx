/** Requests opens on its own queue: a refused read of names, teams or the asker's own access is said in one line and never hides the way to ask. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { MemoryRouter } from 'react-router';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { Requests } from '../src/features/requests/Requests';
import { ShellProvider } from '../src/shell/ShellContext';
import { serve, settle } from './harness';
import { SERVICE, ok, refused } from './fixtures';

const model = { action_sentences: { read: 'Read this resource' }, version: 1, relations: { reader: ['read'] }, kinds: {} };
const roots: ReturnType<typeof createRoot>[] = [];
beforeEach(() => sessionStorage.clear());
afterEach(() => { for (const root of roots.splice(0)) act(() => root.unmount()); document.body.innerHTML = ''; });

async function open(routes: Record<string, Parameters<typeof serve>[0][string]>) {
  serve({ ...SERVICE, '/requests': ok({ requests: [] }), '/grants/model': ok(model), ...routes });
  const container = document.createElement('div'); document.body.append(container);
  const root = createRoot(container); roots.push(root);
  await act(async () => { root.render(<MemoryRouter><ShellProvider><Requests /></ShellProvider></MemoryRouter>); });
  await settle();
  return container;
}

describe('Requests opens on its own queue', () => {
  it('offers the Ask form when the directory of people is refused, and says why names are missing', async () => {
    const page = await open({ '/people': refused(403, 'NotAdmitted', 'people are not listed for you'), '/directory/people': refused(403, 'NotAdmitted', 'people are not listed for you') });
    expect(page.querySelector('form[aria-label="Ask for access"]')).not.toBeNull();
    expect(page.textContent).toContain("People's names are not shown here");
    expect(page.textContent).not.toContain('Lys could not show your access requests');
  });

  it('offers the fields for naming what is needed when the asker\'s own access is refused, with the reason', async () => {
    const page = await open({ '/grants': refused(403, 'NotAdmitted', 'grants are not listed for you') });
    const form = page.querySelector('form[aria-label="Ask for access"]');
    expect(form).not.toBeNull();
    expect(form?.querySelector('input[name="kind"]')).not.toBeNull();
    expect(form?.querySelector('input[name="another"]')).toBeNull();
    expect(form?.textContent).toContain('There is no list of your access to choose from. Lys does not show this account that list.');
  });

  it('still fails whole, in words, when the queue itself is refused', async () => {
    const page = await open({ '/requests': refused(403, 'NotAdmitted', 'requests are closed to you') });
    expect(page.textContent).toContain('Lys could not show your access requests');
  });
});
