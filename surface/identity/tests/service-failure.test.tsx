/** Service failures stay distinct from permission refusals and never retry a write. */
import { describe, expect, it } from 'vitest';
import { mount, text } from './harness';
import { SERVICE, refused } from './fixtures';

describe('Service failure presentation', () => {
  it.each([500, 502, 503, 504])('names a service failure for HTTP %s while retaining its evidence', async (status) => {
    const { posted } = await mount('#/people', { ...SERVICE, '/directory/people': refused(status, 'DirectoryUnavailable', 'The directory could not be read') });
    expect(text()).toContain('The service could not complete this request');
    expect(text()).toContain('DirectoryUnavailable'); expect(text()).toContain('The directory could not be read');
    expect(text()).toContain('keep its original request'); expect(posted).toEqual([]);
  });
  it('keeps a permission refusal distinct from service downtime', async () => {
    await mount('#/people', { ...SERVICE, '/directory/people': refused(403, 'DirectoryPrivate', 'This person cannot read the directory') });
    expect(text()).toContain('Refused'); expect(text()).toContain('DirectoryPrivate');
    expect(text()).not.toContain('The service could not complete this request');
  });
});
