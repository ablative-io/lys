/**
 * A stranger founds an organisation from the public surface (DIRECTORY-111
 * R3): the sign-in page offers it, the founding form posts the three fields
 * once with an operation id, the answer is read as a sentence naming the
 * email and the organisation and never an identity, and the account page
 * says how a proven founder is signed in.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { $, $$, click, mount, text, type, unmountAll } from './harness';
import { ME, SERVICE, ok, refused } from './fixtures';

afterEach(() => unmountAll());

async function fill() {
  await type($('input[name="organisation"]'), "Bea's Bakery");
  await type($('input[name="display_name"]'), 'Bea Baker');
  await type($('input[name="email"]'), 'bea@example.test');
}

describe('Founding an organisation', () => {
  it('is offered on the sign-in page', async () => {
    await mount('#/sign-in', SERVICE);
    const line = $('.found-line');
    expect(line?.textContent).toBe('New here? Found an organisation.');
    expect(line?.querySelector('a')?.getAttribute('href')).toBe('#/found');
  });

  it('posts the three fields and an operation id once, and says Lys wrote to the address', async () => {
    const { posted } = await mount('#/found', {
      ...SERVICE,
      'POST /organisations/found': (body) => {
        const asked = body as { operation: string; email: string };
        return ok({ operation: asked.operation, letter: { letter: 'founded', to: asked.email, sent: true, by: 'lys' } });
      },
    });
    expect($('.page.found h1')?.textContent).toBe('Found an organisation');
    expect($$('form[aria-label="Found an organisation"] .field input')).toHaveLength(3);
    await fill();
    await click($('button[aria-label="Found"]'));
    expect(posted).toHaveLength(1);
    expect(posted[0].path).toBe('/organisations/found');
    const body = posted[0].body as Record<string, unknown>;
    expect(body.organisation).toBe("Bea's Bakery");
    expect(body.display_name).toBe('Bea Baker');
    expect(body.email).toBe('bea@example.test');
    expect(body.operation).toMatch(/^op-[0-9a-f]{32}$/);
    expect(Object.keys(body).sort()).toEqual(['display_name', 'email', 'operation', 'organisation']);
    expect($('[role="status"]')?.textContent).toBe("Lys wrote to bea@example.test. Open the letter and follow its link to prove the address and finish founding Bea's Bakery.");
    expect(text()).not.toMatch(/person-|organisation-[0-9a-f]/);
    expect($('form')).toBeNull();
  });

  it('shows the link when Lys cannot send email and the answer hands it (DIRECTORY-112 R4)', async () => {
    const link = 'https://lys.example.test/#/found/xK9mQ2vLp8Rt4Wn7Yb3Zc6Fh1Jd5Sg0Ae2Uo4Ii8Qw';
    const { posted } = await mount('#/found', {
      ...SERVICE,
      'POST /organisations/found': (body) => {
        const asked = body as { operation: string; email: string };
        return ok({ operation: asked.operation, letter: { letter: 'founded', to: asked.email, sent: false, by: 'lys', reason: 'Lys has no mail settings, so the link is handed here', handed: link } });
      },
    });
    await fill();
    await click($('button[aria-label="Found"]'));
    expect(posted).toHaveLength(1);
    expect($('.page.found h1')?.textContent).toBe('Your link');
    expect($('[role="status"]')?.textContent).toBe("This Lys cannot send email, so your link is here instead of in a letter. Open it to finish founding Bea's Bakery; it works once.");
    const anchor = $('a.handed-link');
    expect(anchor?.textContent).toBe(link);
    expect(anchor?.getAttribute('href')).toBe(link);
    expect(text()).not.toMatch(/person-|organisation-[0-9a-f]/);
    expect($('form')).toBeNull();
  });

  it('says the address already signs in when Lys cannot send email and the answer is already_yours unsent', async () => {
    await mount('#/found', {
      ...SERVICE,
      'POST /organisations/found': (body) => {
        const asked = body as { operation: string; email: string };
        return ok({ operation: asked.operation, letter: { letter: 'already_yours', to: asked.email, sent: false, by: 'lys', reason: 'This address already signs in to Lys' } });
      },
    });
    await fill();
    await click($('button[aria-label="Found"]'));
    const status = $('[role="status"]');
    expect(status?.textContent).toBe('That address already signs in to Lys. Sign in instead.');
    expect(status?.querySelector('a')?.getAttribute('href')).toBe('#/sign-in');
    expect($('form')).toBeNull();
  });

  it('shows a malformed request in its words without the name', async () => {
    await mount('#/found', {
      ...SERVICE,
      'POST /organisations/found': refused(400, 'RequestMalformed', 'RequestMalformed: not-an-email is not an email'),
    });
    await fill();
    await click($('button[aria-label="Found"]'));
    expect($('[role="alert"]')?.textContent).toBe('not-an-email is not an email');
  });

  it('asks for every field before posting', async () => {
    const { posted } = await mount('#/found', SERVICE);
    await click($('button[aria-label="Found"]'));
    expect(posted).toHaveLength(0);
    expect($('[role="alert"]')?.textContent).toBe("Enter the organisation's name, your name and your email.");
  });
});

describe('The account page after a founding', () => {
  it('says how the founder is signed in when the address carries founded=1', async () => {
    await mount('#/settings/account?founded=1', { ...SERVICE, '/me': ok(ME) });
    expect($('.founded-note')?.textContent).toBe("You are signed in by the letter's link. Set a password or connect Google here so you can sign in again.");
  });

  it('says how an admitted person is signed in when the address carries added=1 (DIRECTORY-112 R4)', async () => {
    await mount('#/settings/account?added=1', { ...SERVICE, '/me': ok(ME) });
    expect($('.added-note')?.textContent).toBe('You are signed in by the link you were given. Set a password or connect Google here so you can sign in again.');
    expect($('.founded-note')).toBeNull();
  });

  it('says nothing of a letter or a link otherwise', async () => {
    await mount('#/settings/account', { ...SERVICE, '/me': ok(ME) });
    expect($('section[aria-label="Your account"]')).not.toBeNull();
    expect($('.founded-note')).toBeNull();
    expect($('.added-note')).toBeNull();
  });
});
