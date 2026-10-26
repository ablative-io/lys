/**
 * The proving page (DIRECTORY-111 R3): the letter's link lands on
 * #/found/<token>, nothing is posted on load, the one button posts the
 * proof once, success lands on the account page told how the person is
 * signed in, and a refusal is read in words with the way to sign-in.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { $, click, mount, settle, unmountAll } from './harness';
import { ME, SERVICE, ok, refused } from './fixtures';

afterEach(() => unmountAll());

const TOKEN = 'xK9mQ2vLp8Rt4Wn7Yb3Zc6Fh1Jd5Sg0Ae2Uo4Ii8Qw';

describe('Finishing a founding by its link', () => {
  it('posts nothing on load and the proof once on the click, then lands on the account page', async () => {
    const { posted } = await mount('#/found/' + TOKEN, {
      ...SERVICE,
      ['/organisations/found/' + TOKEN + '/whom']: ok({ display_name: 'Bea', masked_email: 'b…@example.test' }),
      '/me': ok(ME),
      'POST /organisations/found/prove': ok({ signed_in: { issuer: 'https://issuer.test', subject: 'subject-bea' }, authority: 'a', organisation: 'organisation-' + '1'.repeat(32), person: 'person-' + '2'.repeat(32) }),
    });
    expect($('.page.found h1')?.textContent).toBe('Finish founding');
    expect(posted).toHaveLength(0);
    await click($('button[aria-label="I am Bea, sign me in"]'));
    await settle();
    expect(posted).toEqual([{ path: '/organisations/found/prove', body: { proof: TOKEN } }]);
    expect(location.hash).toBe('#/settings/account?founded=1');
    expect($('.founded-note')?.textContent).toBe("You are signed in by the letter's link. Set a password or connect Google here so you can sign in again.");
  });

  it('shows a refused proof in its words with the way to sign in', async () => {
    const { posted } = await mount('#/found/' + TOKEN, {
      ...SERVICE,
      ['/organisations/found/' + TOKEN + '/whom']: ok({ display_name: 'Bea', masked_email: 'b…@example.test' }),
      'POST /organisations/found/prove': refused(403, 'proof_refused', 'proof_refused: This link is not one Lys issued, or it was already used. Found again from the sign-in page.'),
    });
    await click($('button[aria-label="I am Bea, sign me in"]'));
    expect(posted).toHaveLength(1);
    const alert = $('[role="alert"]');
    expect(alert?.textContent).toBe('This link is not one Lys issued, or it was already used. Found again from the sign-in page. Go to sign in.');
    expect(alert?.querySelector('a')?.getAttribute('href')).toBe('#/sign-in');
    expect(location.hash).toBe('#/found/' + TOKEN);
  });

  it('shows already_admitted in its words with the way to sign in', async () => {
    await mount('#/found/' + TOKEN, {
      ...SERVICE,
      ['/organisations/found/' + TOKEN + '/whom']: ok({ display_name: 'Bea', masked_email: 'b…@example.test' }),
      'POST /organisations/found/prove': refused(403, 'already_admitted', 'already_admitted: That address already signs in to Lys. Sign in there; nothing was founded.'),
    });
    await click($('button[aria-label="I am Bea, sign me in"]'));
    const alert = $('[role="alert"]');
    expect(alert?.textContent).toBe('That address already signs in to Lys. Sign in there; nothing was founded. Go to sign in.');
    expect(alert?.querySelector('a')?.getAttribute('href')).toBe('#/sign-in');
  });
});

describe('Signing in by a handed link (DIRECTORY-112 R4)', () => {
  it('posts nothing on load and the proof once on the click, then lands on the account page told of the link', async () => {
    const { posted } = await mount('#/added/' + TOKEN, {
      ...SERVICE,
      ['/organisations/found/' + TOKEN + '/whom']: ok({ display_name: 'Bea', masked_email: 'b…@example.test' }),
      '/me': ok(ME),
      'POST /organisations/found/prove': ok({ signed_in: { issuer: 'https://issuer.test', subject: 'subject-bea' }, authority: 'a', organisation: 'organisation-' + '1'.repeat(32), person: 'person-' + '2'.repeat(32) }),
    });
    expect($('.page.found h1')?.textContent).toBe('Sign in to Lys');
    expect($('.page.found .sub')?.textContent).toBe('This link is for Bea (b…@example.test)');
    expect(posted).toHaveLength(0);
    await click($('button[aria-label="I am Bea, sign me in"]'));
    await settle();
    expect(posted).toEqual([{ path: '/organisations/found/prove', body: { proof: TOKEN } }]);
    expect(location.hash).toBe('#/settings/account?added=1');
    expect($('.added-note')?.textContent).toBe('You are signed in by the link you were given. Set a password or connect Google here so you can sign in again.');
  });

  it('shows a refused link in its words with the way to sign in', async () => {
    await mount('#/added/' + TOKEN, {
      ...SERVICE,
      ['/organisations/found/' + TOKEN + '/whom']: ok({ display_name: 'Bea', masked_email: 'b…@example.test' }),
      'POST /organisations/found/prove': refused(403, 'proof_refused', 'proof_refused: This link is not one Lys issued, or it was already used. Found again from the sign-in page.'),
    });
    await click($('button[aria-label="I am Bea, sign me in"]'));
    const alert = $('[role="alert"]');
    expect(alert?.textContent).toBe('This link is not one Lys issued, or it was already used. Found again from the sign-in page. Go to sign in.');
    expect(alert?.querySelector('a')?.getAttribute('href')).toBe('#/sign-in');
  });
});

describe('Going on to the product that asked (DIRECTORY-113 R3)', () => {
  const PROVED = { signed_in: { issuer: 'https://issuer.test', subject: 'subject-bea' }, authority: 'a', organisation: 'organisation-' + '1'.repeat(32), person: 'person-' + '2'.repeat(32) };

  it('offers the way on to the origin the proof answers, carried in navigation state', async () => {
    await mount('#/added/' + TOKEN, {
      ...SERVICE,
      ['/organisations/found/' + TOKEN + '/whom']: ok({ display_name: 'Bea', masked_email: 'b…@example.test' }),
      '/me': ok(ME),
      'POST /organisations/found/prove': ok({ ...PROVED, continue_to: 'https://cambium.example.test' }),
    });
    await click($('button[aria-label="I am Bea, sign me in"]'));
    await settle();
    expect(location.hash).toBe('#/settings/account?added=1');
    const onward = $('.added-note a');
    expect(onward?.textContent).toBe('Continue to cambium.example.test');
    expect(onward?.getAttribute('href')).toBe('https://cambium.example.test');
  });

  it('offers no way on when the proof answers none', async () => {
    await mount('#/added/' + TOKEN, { ...SERVICE,
      ['/organisations/found/' + TOKEN + '/whom']: ok({ display_name: 'Bea', masked_email: 'b…@example.test' }), '/me': ok(ME), 'POST /organisations/found/prove': ok(PROVED) });
    await click($('button[aria-label="I am Bea, sign me in"]'));
    await settle();
    expect($('.added-note')).not.toBeNull();
    expect($('.added-note a')).toBeNull();
  });

  it('offers no way on from the address alone, whatever it carries', async () => {
    await mount('#/settings/account?added=1&continueTo=https://elsewhere.example.test', { ...SERVICE,
      ['/organisations/found/' + TOKEN + '/whom']: ok({ display_name: 'Bea', masked_email: 'b…@example.test' }), '/me': ok(ME) });
    await settle();
    expect($('.added-note')).not.toBeNull();
    expect($('.added-note a')).toBeNull();
  });
});

describe('Whose link this is (DIRECTORY-122)', () => {
  it("names the record before the act and This isn't me posts nothing", async () => {
    const { posted } = await mount('#/added/' + TOKEN, { ...SERVICE,
      ['/organisations/found/' + TOKEN + '/whom']: ok({ display_name: 'Dean Whiting', masked_email: 'd…@ablative.com.au' }),
    });
    expect(textOf('.page.found .sub')).toBe('This link is for Dean Whiting (d…@ablative.com.au)');
    expect($('button[aria-label="I am Dean Whiting, sign me in"]')).not.toBeNull();
    await click($('button[aria-label="This isn\'t me"]'));
    expect(posted).toEqual([]);
    expect($('.page.found')?.textContent).toContain('Ask the person who sent this link');
    expect($('button[aria-label="I am Dean Whiting, sign me in"]')).toBeNull();
  });
  it('names both people when another person is already signed in', async () => {
    await mount('#/added/' + TOKEN, { ...SERVICE,
      ['/organisations/found/' + TOKEN + '/whom']: ok({ display_name: 'Dean Whiting', masked_email: 'd…@ablative.com.au' }),
      'POST /organisations/found/prove': refused(409, 'SignedInAsAnother', 'SignedInAsAnother: You are signed in as Annabel. This link is for Dean Whiting.'),
    });
    await click($('button[aria-label="I am Dean Whiting, sign me in"]'));
    expect($('[role="alert"]')?.textContent).toContain('You are signed in as Annabel. This link is for Dean Whiting.');
  });
});
function textOf(selector: string) { return $(selector)?.textContent; }

it('names the provider and both masked addresses after a refused provider join', async () => {
  await mount('#/sign-in?refused=ProviderEmailMismatch&provider=Google&account_email=a%E2%80%A6%40example.test&held_email=d%E2%80%A6%40example.test', SERVICE);
  expect($('.page.sign-in')?.textContent).toContain('Google returned the sign-in account a…@example.test, but Lys holds d…@example.test for this person');
});
