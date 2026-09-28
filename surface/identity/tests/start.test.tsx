/**
 * DIRECTORY-029 R13: the Start drawer and the unconfirmed notice on the
 * agent file. CONFORMANCE 5.3 (no credential value on the command line or
 * the clipboard), 5.5 (copying changes no state; running only on a report)
 * and 5.6 (unconfirmed, never 'not started', the request stands).
 */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { $, click, mount, serve, settle, text } from './harness';
import { SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { StartDrawer } from '../src/features/start/StartDrawer';
import { START_CHECKS } from '../src/generated';
import { AGENT, GIVEN, L1, RUNNING_WITHDRAWN, SUSPENDED, UNCONFIRMED, VALUES, WITHDRAWN, answer, installClipboard, saysNotStarted } from './acceptance/start.spec';

const START = 'POST /agents/' + AGENT + '/start';
const STATE = '/launch-records/' + L1 + '/state';
const WITHDRAW = 'POST /launch-records/' + L1 + '/withdraw';
/** The directory scope answers the administrator; the personal scope anyone else. */
const ADMIN = { '/directory/people': ok({ scope: 'directory', people: [] }) };
const OTHER = { '/directory/people': refused(403, 'NotAdmitted', 'only the configured administrator may do this in step 1'), '/people': ok({ scope: 'personal', people: [] }) };

let root: Root | null = null;
const seen: string[] = [];

async function open(routes: Record<string, Route>) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve(routes, posted);
  const element = document.createElement('div');
  document.body.appendChild(element);
  root = createRoot(element);
  await act(async () => { root?.render(<StartDrawer agent={AGENT} />); });
  await settle();
  seen.push(text());
  return { posted, requests };
}

function close() { act(() => root?.unmount()); root = null; document.body.innerHTML = ''; }

const button = (name: string) => $('[data-act="' + name + '"]');

async function ask() {
  await click(button('start'));
  seen.push(text());
  await click(button('ask-start'));
  seen.push(text());
}

afterEach(close);

describe('the Start drawer', () => {
  it('gives the administrator an enabled Start whose drawer lists the five checks in order', async () => {
    await open(ADMIN);
    const start = button('start');
    expect(start).not.toBeNull();
    expect((start as HTMLButtonElement).disabled).toBe(false);
    await click(start);
    const names = [...document.querySelectorAll('[data-check]')].map((item) => item.getAttribute('data-check'));
    expect(names).toEqual([...START_CHECKS]);
    expect(names).toHaveLength(5);
  });

  it('shows anyone else no enabled Start and the refusal naming the agent and the right', async () => {
    await open(OTHER);
    const start = button('start') as HTMLButtonElement | null;
    expect(start === null || start.disabled).toBe(true);
    expect(text()).toContain(AGENT);
    expect(text()).toContain('start');
    expect($('[data-command]')).toBeNull();
  });

  it('copies exactly the command shown, holding no credential value, and sends nothing', async () => {
    const clipboard = installClipboard();
    const { requests } = await open({ ...ADMIN, [START]: answer(200, GIVEN) });
    await ask();
    const shown = $('[data-command]')?.textContent ?? '';
    expect(shown).toBe(GIVEN.command);
    const before = requests.length;
    await click(button('copy'));
    seen.push(text());
    const copied = await navigator.clipboard.readText();
    expect(copied).toBe(shown);
    expect(clipboard.written).toEqual([shown]);
    let checked = 0;
    for (const value of VALUES) {
      expect(copied).not.toContain(value);
      expect(shown).not.toContain(value);
      expect(text()).not.toContain(value);
      checked += 1;
    }
    expect(checked).toBe(2);
    // CONFORMANCE 5.5: pressing Copy sends no request, and the record still reads unconfirmed.
    expect(requests.length).toBe(before);
    expect(requests.filter((path) => path.startsWith('POST ')).length).toBe(1);
    expect($('[data-launch-state]')?.getAttribute('data-launch-state')).toBe('unconfirmed');
    expect(text()).toContain('Unconfirmed');
    expect($('[data-working-directory]')?.textContent).toBe('fixture-cwd');
  });

  it('shows a suspended agent failing the agent is active, with no command and no Copy', async () => {
    await open({ ...ADMIN, [START]: answer(409, SUSPENDED) });
    await ask();
    const active = $('[data-check="the agent is active"]')?.textContent ?? '';
    expect(active).toContain('the agent is active');
    expect(active).toContain('failed');
    expect($('[data-command]')).toBeNull();
    expect(button('copy')).toBeNull();
    expect(text()).toContain('agent_not_active');
  });
});

describe('the unconfirmed notice', () => {
  it('says the request stands and warns against asking elsewhere', async () => {
    await open({ ...ADMIN, [START]: answer(200, GIVEN) });
    await ask();
    const notice = $('[data-launch-state]')?.textContent ?? '';
    for (const words of ['Unconfirmed', 'The request stands', 'could start it twice', 'wait for its report', 'withdraw']) {
      expect(notice).toContain(words);
    }
  });

  it('reads withdrawn after a withdrawal, then running with the withdrawal beside it', async () => {
    let reported = false;
    const routes: Record<string, Route> = {
      ...ADMIN,
      [START]: answer(200, GIVEN),
      [WITHDRAW]: answer(200, WITHDRAWN),
      [STATE]: () => answer(200, reported ? RUNNING_WITHDRAWN : UNCONFIRMED),
    };
    const { posted } = await open(routes);
    await ask();
    await click(button('withdraw'));
    seen.push(text());
    expect(posted.map((entry) => entry.path)).toContain('/launch-records/' + L1 + '/withdraw');
    expect($('[data-launch-state]')?.getAttribute('data-launch-state')).toBe('withdrawn');
    expect(text()).toContain('withdrawn');
    reported = true;
    await click(button('read-state'));
    seen.push(text());
    expect($('[data-launch-state]')?.getAttribute('data-launch-state')).toBe('running');
    expect($('[data-withdrawal]')?.textContent).toContain('withdrawn by admin-fixture');
  });
});

describe('the agent file route', () => {
  it('opens the Start drawer at the agent file\'s start address', async () => {
    await mount('#/file/' + AGENT + '/start', { ...SERVICE, ...ADMIN });
    seen.push(text());
    expect(text()).toContain('Start ' + AGENT);
    expect(button('start')).not.toBeNull();
  });
});

describe('every page the tests reached', () => {
  it('never says not started', () => {
    expect(seen.length).toBeGreaterThan(10);
    for (const page of seen) expect(saysNotStarted(page)).toBe(false);
  });
});
