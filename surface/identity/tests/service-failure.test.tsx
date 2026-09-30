/** Service failures stay distinct from permission refusals and never retry a write. */
import { describe, expect, it } from 'vitest';
import { mount, text } from './harness';
import { SERVICE, refused } from './fixtures';

function errorDetails() {
  const detail = [...document.querySelectorAll('details')].find((entry) => entry.querySelector('summary')?.textContent === 'Error details');
  if (!detail) throw new Error('The error details are missing');
  expect(detail.open).toBe(false);
  return detail;
}

describe('Service failure presentation', () => {
  it.each([500, 502, 503, 504])('names a service failure for HTTP %s while retaining its evidence', async (status) => {
    const { posted } = await mount('#/people', { ...SERVICE, '/directory/people': refused(status, 'DirectoryUnavailable', 'The directory could not be read') });
    expect(text()).toContain('This part of Lys cannot answer.');
    expect(text()).toContain('Ask the administrator to check the service.');
    expect(text()).toContain('Keep any change whose outcome is unknown.');
    expect(errorDetails().textContent).toContain('DirectoryUnavailable: The directory could not be read');
    expect(posted).toEqual([]);
  });
  it('keeps a permission refusal distinct from service downtime', async () => {
    await mount('#/people', { ...SERVICE, '/directory/people': refused(403, 'DirectoryPrivate', 'This person cannot read the directory') });
    expect(text()).toContain('Ask the administrator to check the error details before repeating a change.');
    expect(errorDetails().textContent).toContain('DirectoryPrivate: This person cannot read the directory');
    expect(text()).not.toContain('This part of Lys cannot answer.');
  });
});
