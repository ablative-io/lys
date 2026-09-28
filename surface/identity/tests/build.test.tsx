/** The sign-in view that lists the authority names the build the service says it runs. */
import { describe, expect, it } from 'vitest';
import { mount, text } from './harness';
import { BUILD, SERVICE, ok, refused } from './fixtures';

describe('Running build', () => {
  it('shows the commit from the /authority answer beside the authority', async () => {
    const { requests } = await mount('#/people', { ...SERVICE, '/directory/people': refused(401, 'NotSignedIn', 'Sign in first') });
    expect(requests).toContain('/authority');
    expect(text()).toContain('Step 1 of the directory has one administrator.');
    expect(text()).toContain('Build ' + BUILD);
  });

  it('shows the words of a build with no commit exactly as the service says them', async () => {
    const words = 'not built from a git commit';
    await mount('#/people', {
      ...SERVICE,
      '/directory/people': refused(401, 'NotSignedIn', 'Sign in first'),
      '/authority': ok({ authority: 'Step 1 of the directory has one administrator.', build: words }),
    });
    expect(text()).toContain('Build ' + words);
    expect(text()).not.toContain(BUILD);
  });
});
