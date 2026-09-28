// @vitest-environment jsdom
/**
 * R7's browser acceptance: the why view, driven as a person drives it,
 * against the standalone identity server with its step-2 engine, and every
 * verdict and reason it shows compared with the server's own answer.
 *
 * crates/lys-identity-server/tests/why_acceptance.rs runs this file under
 * vitest.acceptance.config.ts, as part of the Rust suite. It starts the
 * server over a disposable SpiceDB and sets up the fixture, extending R6's:
 * person P and a second person Q, agent A and identity B; the root authority
 * issues P a root grant on project X and Q one on project Y; P grants A read
 * on X (grant PA); Q grants B read on Y (grant QB); B holds nothing on X. It
 * names the server, a signed-in session of the root authority, who may see
 * A and B, and the fixture's ids, as JSON in LYS_WHY_ACCEPTANCE:
 *
 *   {"service": "http://127.0.0.1:8411", "session": "lys_session=...",
 *    "agent_a": "agent-...", "identity_b": "agent-...", "person_p": "person-...",
 *    "resource_x": {"kind": "project", "id": "x"}}
 *
 * Without LYS_WHY_ACCEPTANCE it fails, saying so.
 */
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it, vi } from 'vitest';
import { App } from '../../src/App';
import { $, $$, choose, click, settle } from '../harness';

interface Fixture {
  service: string;
  session: string;
  agent_a: string;
  identity_b: string;
  person_p: string;
  resource_x: { kind: string; id: string };
}

interface Explanation {
  permitted: boolean;
  reason: string | null;
  path: { identity: string; responsible: boolean }[];
}

function fixture(): Fixture {
  const env = (globalThis as { process?: { env: Record<string, string | undefined> } }).process?.env;
  const raw = env?.LYS_WHY_ACCEPTANCE;
  if (!raw) throw new Error('LYS_WHY_ACCEPTANCE names no standalone server and fixture; run crates/lys-identity-server/tests/why_acceptance.rs');
  return JSON.parse(raw) as Fixture;
}

/** The server's own answer to the question, asked directly. */
async function serverAnswer(f: Fixture, identity: string): Promise<Explanation> {
  const response = await fetch(f.service + '/grants/explain', {
    method: 'POST',
    headers: { 'content-type': 'application/json', cookie: f.session },
    body: JSON.stringify({ identity, resource: f.resource_x, action: 'read' }),
  });
  expect(response.status).toBe(200);
  return (await response.json()) as Explanation;
}

/**
 * Send the page's requests to the standalone server with the session, each
 * answer read whole before the page sees it, and keep the ones in flight.
 */
function forward(f: Fixture): Promise<Response>[] {
  const real = globalThis.fetch.bind(globalThis);
  const inFlight: Promise<Response>[] = [];
  vi.stubGlobal('fetch', (input: string, init?: RequestInit) => {
    const path = String(input).replace(/^\/api/, '');
    const headers = { ...(init?.headers as Record<string, string> | undefined), cookie: f.session };
    const sent = real(f.service + path, { ...init, headers }).then(async (answer) => {
      const body = await answer.text();
      return new Response(body, { status: answer.status, headers: answer.headers });
    });
    inFlight.push(sent);
    return sent;
  });
  return inFlight;
}

/**
 * Let the page's requests land until `shown` holds. It stops by its signal:
 * when nothing is in flight and the page still does not show it, it fails.
 */
async function until(inFlight: Promise<Response>[], shown: () => boolean, what: string): Promise<void> {
  for (;;) {
    if (shown()) return;
    const pending = inFlight.splice(0);
    if (pending.length > 0) {
      await Promise.all(pending);
      await settle();
      continue;
    }
    await settle();
    if (shown()) return;
    if (inFlight.length === 0) throw new Error(`nothing is in flight and the page does not show ${what}`);
  }
}

/**
 * Open the why view for `identity`, ask about read on X, and wait for the
 * server's answer to show. The view stays mounted so its answer can be read;
 * the returned function takes it down and gives back the real fetch.
 */
async function askInView(f: Fixture, identity: string): Promise<() => void> {
  const inFlight = forward(f);
  const x = `${f.resource_x.kind}:${f.resource_x.id}`;
  location.hash = '#/file/' + identity + '/why';
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => {
    root.render(createElement(App));
  });
  await until(inFlight, () => $(`#whyRes option[value="${x}"]`) !== null, `${x} to ask about`);
  await choose($('#whyRes'), x);
  await choose($('#whyAction'), 'read');
  await click($('[data-act="why"]'));
  await until(inFlight, () => $('[data-verdict]') !== null || $('[data-refusal]') !== null, 'an answer');
  return () => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  };
}

describe('The why view against the standalone server (R7)', () => {
  it('shows the server’s yes for A with the path A, P and P responsible, and its no for B with no_grant', async () => {
    const f = fixture();
    let asked = 0;

    const forA = await serverAnswer(f, f.agent_a);
    asked += 1;
    expect(forA.permitted).toBe(true);
    expect(forA.path.map((step) => step.identity)).toEqual([f.agent_a, f.person_p]);
    expect(forA.path.map((step) => step.responsible)).toEqual([false, true]);
    const closeA = await askInView(f, f.agent_a);
    expect($('[data-state="refusal"]')?.textContent ?? null).toBeNull();
    expect($('[data-verdict]')?.dataset.verdict).toBe('yes');
    const steps = $$('[data-responsible]');
    expect(steps.map((li) => (li.textContent ?? '').split(' ')[0])).toEqual(forA.path.map((step) => step.identity));
    expect(steps.map((li) => li.dataset.responsible)).toEqual(forA.path.map((step) => String(step.responsible)));
    expect($('[data-responsible="true"]')?.textContent).toContain(f.person_p);
    closeA();
    expect($('[data-verdict]')).toBeNull();

    const forB = await serverAnswer(f, f.identity_b);
    asked += 1;
    expect(forB.permitted).toBe(false);
    expect(forB.reason).toBe('no_grant');
    const closeB = await askInView(f, f.identity_b);
    expect($('[data-state="refusal"]')?.textContent ?? null).toBeNull();
    expect($('[data-verdict]')?.dataset.verdict).toBe('no');
    expect($('[data-reason]')?.textContent).toBe(forB.reason);
    closeB();
    expect(asked).toBe(2);
  });
});
