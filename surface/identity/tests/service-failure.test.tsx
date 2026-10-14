/** Service failures stay distinct from permission refusals and never retry a write. */
import { describe, expect, it } from 'vitest';
import { mount, text } from './harness';
import { SERVICE, refused } from './fixtures';

function errorDetails() {
  const detail = document.querySelector('.why-not details .refusal-name');
  if (!detail) throw new Error('The error details are missing');
  // The service's own words are kept whole under closed Error details, below the sentence (14 Oct refusal words).
  expect(detail.closest('details')?.hasAttribute('open')).toBe(false);
  return detail;
}

describe('Service failure presentation', () => {
  it.each([500, 502, 503, 504])('names a service failure for HTTP %s while retaining its evidence', async (status) => {
    const { posted } = await mount('#/people', { ...SERVICE, '/directory/people': refused(status, 'DirectoryUnavailable', 'The directory could not be read') });
    expect(document.querySelector('.why-not > p')?.textContent).toBe('Lys could not read its directory. Ask the administrator to check the service.');
    expect(errorDetails().textContent).toContain('DirectoryUnavailable: The directory could not be read');
    expect(posted).toEqual([]);
  });
  it('keeps a permission refusal distinct from service downtime', async () => {
    await mount('#/people', { ...SERVICE, '/directory/people': refused(403, 'DirectoryPrivate', 'This person cannot read the directory') });
    expect(document.querySelector('.why-not > p')?.textContent).toBe('Lys refused: This person cannot read the directory');
    expect(errorDetails().textContent).toContain('DirectoryPrivate: This person cannot read the directory');
    expect(text()).not.toContain('Lys could not read its directory.');
  });
});
