import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { Gate } from '../src/features/signin/Gate';

describe('Loading words', () => {
  it.each(['Secrets', 'Settings', 'Roles', 'Personal budgets'])('names %s while its read is pending', (title) => {
    const html = renderToStaticMarkup(<Gate load={{ status: 'loading' }} title={title} ok={() => null} />);
    expect(html).toContain('Reading ' + title + '…');
    expect(html).not.toContain('Reading the directory');
  });
});
