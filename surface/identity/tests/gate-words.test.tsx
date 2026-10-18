import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { Callback, Gate, Loading } from '../src/features/signin/Gate';
import { $, mount, text } from './harness';
import { SERVICE, refused } from './fixtures';

describe('Loading words', () => {
  it.each(['Secrets', 'Settings', 'Roles', 'Personal budgets'])('names %s while its read is pending', (title) => {
    const html = renderToStaticMarkup(<Gate load={{ status: 'loading' }} title={title} ok={() => null} />);
    expect(html).toContain('Reading ' + title + '…');
    expect(html).not.toContain('Reading the directory');
  });
});

describe('Loading words that are sentences', () => {
  it('uses a page’s own sentence where its title would not make one', () => {
    const html = renderToStaticMarkup(<Gate load={{ status: 'loading' }} title="You" reading="Reading your account…" ok={() => null} />);
    expect(html).toContain('Reading your account…');
    expect(html).not.toContain('Reading You');
  });

  it('opens Lys and finishes a sign-in in their own words', () => {
    expect(renderToStaticMarkup(<Loading />)).toContain('Opening Lys…');
    expect(renderToStaticMarkup(<Callback />)).toContain('Finishing sign-in…');
  });

  it('reads You as "your account" and names the page You above a failure', async () => {
    await mount('#/me', { ...SERVICE, '/me': refused(503, 'DirectoryUnavailable', 'directory unavailable') });
    expect(text()).not.toContain('Signed in as');
    expect($('.page .eyebrow')?.textContent).toBe('You');
  });
});
