// Domain 04 item 2: the address Cambium is told to call Lys back at is shown before the browser leaves Lys, and the one
// combination that cannot work (Lys open on an address only its own computer answers, Cambium on another) is said in words.
// Said, never enforced: Continue stays available, because a wrong refusal that stops a person is the worse failure.
import { expect, it } from 'vitest';
import { click, mount, type } from '../../../tests/harness';
import { SERVICE, ok } from '../../../tests/fixtures';
import { onlyHere, strandedWhy } from './ConnectApp';

const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === label) ?? null;

it('reads an address as this computer only for localhost, .localhost, 127.0.0.0/8 and ::1, and for nothing else', () => {
  const here = ['http://localhost:8490', 'http://LOCALHOST', 'http://lys.localhost:5184', 'http://127.0.0.1:8490', 'http://127.4.5.6', 'http://[::1]:8490'];
  const elsewhere = ['https://lys.example.com', 'http://192.168.50.40:8490', 'http://100.67.104.58:8490', 'https://localhost.example.com', 'http://128.0.0.1', 'not an address'];
  expect(here.filter(onlyHere)).toEqual(here);
  expect(elsewhere.filter(onlyHere)).toEqual([]);
});

it('says the hand-off will most likely fail only when Lys is here alone and the app is elsewhere', () => {
  expect(strandedWhy('Cambium', 'http://localhost:8490', 'https://cambium.example.com')).toContain('will most likely not reach it');
  expect(strandedWhy('Cambium', 'http://localhost:8490', 'http://localhost:4000')).toBeNull();
  expect(strandedWhy('Cambium', 'https://lys.example.com', 'https://cambium.example.com')).toBeNull();
  expect(strandedWhy('Cambium', 'https://lys.example.com', 'http://localhost:4000')).toBeNull();
});

async function open() {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [] }) });
  await click(button('Connect Cambium'));
  return document.querySelector<HTMLInputElement>('input[aria-label="Cambium web address"]');
}

it('names the address Cambium will call back when Cambium is on the same computer as this browser’s Lys', async () => {
  // jsdom opens the page at a localhost address, so this is the one-computer setup.
  expect(onlyHere(window.location.origin)).toBe(true);
  await type(await open(), 'http://localhost:4000');
  const row = document.querySelector('[aria-label="Known app cambium"]');
  expect(row?.textContent).toContain('Cambium will call Lys back at ' + window.location.origin + '.');
  expect(row?.querySelector('.app-replace')).toBeNull();
});

it('warns, and still lets the person go on, when Cambium is on another computer and Lys is open here alone', async () => {
  await type(await open(), 'https://cambium.example.com');
  const warning = document.querySelector('[aria-label="Known app cambium"] .app-replace');
  expect(warning?.textContent).toContain('Lys is open here at ' + window.location.origin + ', which only this computer answers');
  expect(warning?.textContent).toContain('Open Lys at an address Cambium’s computer can reach');
  expect(document.body.textContent).not.toContain('Cambium will call Lys back at');
  expect(button('Continue to Cambium')?.hasAttribute('disabled')).toBe(false);
});
