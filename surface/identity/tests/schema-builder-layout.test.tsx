import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Readable, Writable } from 'node:stream';
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { App } from '../src/App';
import { KindNode } from '../src/features/apps/kind-builder';
import { GRANTS, LEDGER_G, MODEL, SCRIBE, SERVICE, ok } from './fixtures';
import { serve } from './harness';

const kind = 'cambium.record';
const action = 'read_clinical_records_for_all_assigned_wards_and_departments';
const resource = 'clinical_records_for_the_department_and_every_assigned_ward_'.repeat(3);
const schema = { kinds: { [kind]: { actions: [action], relations: { reader: [action] }, parents: [], hot: [action], agents: [action], roles: {} } } };
const grant = { ...GRANTS[0], resource: { kind, id: 'general' }, relation: 'reader', actions: [action], pass_on: { kind: 'to' as const, actions: [action], recipients: ['agent' as const] } };
const second = { ...grant, id: LEDGER_G, resource: { kind, id: resource } };
const app = { id: 'cambium', name: 'Cambium', state: 'approved', redirects: [], sign_in: null, schema, version: 1, versions: [1], pending: null, client_id: null, service_account: null, client_credentials: [], registered_by: { kind: 'start' }, registered_at: 1 };
const styles = ['features/apps/apps.css', 'features/apps/schema-builder.css', 'features/apps/schema-matrix.css', 'features/file/file.css', 'features/grants/grants.css', 'styles/tokens.css', 'styles/rail.css', 'styles/screen.css', 'styles/forms.css', 'styles/overlays.css', 'styles/dock.css']
  .map((path) => readFileSync(new URL('../src/' + path, import.meta.url), 'utf8'))
  .join('\n');
const initialWidth = innerWidth;
let root: Root;
let style: HTMLStyleElement;

beforeEach(() => {
  sessionStorage.clear();
  style = document.createElement('style');
  style.textContent = styles;
  document.head.appendChild(style);
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  serve({
    ...SERVICE,
    '/apps': ok({ apps: [app] }),
    '/roles': ok({ roles: [] }),
    '/grants': ok({ grants: [grant, second], revision: 7 }),
    '/grants/model': ok({ ...MODEL, kinds: { [kind]: { agents: [action], relations: { reader: [action] } } } }),
    [`/grants/cannot-give?route=browser&source=${second.id}&recipient=${SCRIBE}`]: ok({ source: second.id, recipient: SCRIBE, items: [] }),
    'POST /apps/cambium/schema/check': ok({ current: 1, next: 2, applies: true, stranded: [], diff: { kinds_added: [], kinds_removed: [], relations_added: [], relations_removed: [], actions_added: [], actions_removed: [], parents_added: [], parents_removed: [] } }),
  });
});

afterEach(() => {
  act(() => root.unmount());
  style.remove();
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: initialWidth });
});

function element<T extends HTMLElement>(selector: string): T {
  const found = document.querySelector<T>(selector);
  if (!found) throw new Error('Missing layout subject: ' + selector);
  return found;
}

async function click(selector: string) {
  await act(async () => {
    element(selector).click();
  });
}

async function browserDocument(browser: string, fixture: string, width: number, profile: string): Promise<string> {
  const child = spawn(
    browser,
    [
      '--headless=new',
      '--remote-debugging-pipe',
      '--disable-background-networking',
      '--disable-component-update',
      '--disable-sync',
      '--disable-default-apps',
      '--disable-breakpad',
      '--disable-crash-reporter',
      '--no-first-run',
      '--no-default-browser-check',
      '--use-mock-keychain',
      '--password-store=basic',
      '--user-data-dir=' + profile,
    ],
    { stdio: ['ignore', 'ignore', 'pipe', 'pipe', 'pipe'] }
  );
  let stopping = false,
    diagnostic = '';
  const pending = new Map<number, { resolve: (value: unknown) => void; reject: (problem: Error) => void }>();
  const events = new Map<string, { resolve: () => void; reject: (problem: Error) => void }>();
  let fatal: Error | null = null;
  const failed = (problem: Error) => {
    fatal = problem;
    for (const entry of pending.values()) entry.reject(problem);
    pending.clear();
    for (const entry of events.values()) entry.reject(problem);
    events.clear();
  };
  child.stderr?.on('data', (chunk: Buffer) => {
    diagnostic += chunk.toString();
  });
  child.on('error', failed);
  const closed = new Promise<void>((resolve) =>
    child.once('close', (code, signal) => {
      if (!stopping) failed(new Error('Chrome closed before layout was read: ' + String(code) + '/' + String(signal) + '\n' + diagnostic));
      resolve();
    })
  );
  try {
    const input = child.stdio[3],
      output = child.stdio[4];
    if (!(input instanceof Writable) || !(output instanceof Readable)) throw new Error('Chrome did not open its protocol pipes');
    input.on('error', failed);
    output.setEncoding('utf8');
    let buffer = '',
      nextId = 0;
    output.on('data', (chunk: string) => {
      buffer += chunk;
      let end = buffer.indexOf('\0');
      while (end >= 0) {
        const packet = buffer.slice(0, end);
        buffer = buffer.slice(end + 1);
        const message = JSON.parse(packet) as { id?: number; result?: unknown; error?: { message: string }; method?: string; sessionId?: string };
        if (message.id !== undefined) {
          const entry = pending.get(message.id);
          pending.delete(message.id);
          if (message.error) entry?.reject(new Error(message.error.message));
          else entry?.resolve(message.result);
        } else if (message.method) {
          const key = (message.sessionId ?? '') + ':' + message.method;
          events.get(key)?.resolve();
          events.delete(key);
        }
        end = buffer.indexOf('\0');
      }
    });
    const call = <T,>(method: string, params: Record<string, unknown> = {}, sessionId?: string) =>
      new Promise<T>((resolve, reject) => {
        if (fatal) {
          reject(fatal);
          return;
        }
        const id = ++nextId;
        pending.set(id, { resolve: (value) => resolve(value as T), reject });
        input.write(JSON.stringify({ id, method, params, sessionId }) + '\0');
      });
    const { targetId } = await call<{ targetId: string }>('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await call<{ sessionId: string }>('Target.attachToTarget', { targetId, flatten: true });
    await call('Page.enable', {}, sessionId);
    await call('Emulation.setDeviceMetricsOverride', { width, height: 1000, deviceScaleFactor: 1, mobile: false }, sessionId);
    const loaded = new Promise<void>((resolve, reject) => events.set(sessionId + ':Page.loadEventFired', { resolve, reject }));
    const navigation = await call<{ errorText?: string }>('Page.navigate', { url: new URL('file://' + fixture).href }, sessionId);
    if (navigation.errorText) throw new Error('Chrome navigation failed: ' + navigation.errorText);
    await loaded;
    const answer = await call<{ result: { value?: string }; exceptionDetails?: unknown }>('Runtime.evaluate', { expression: 'document.fonts.ready.then(() => document.documentElement.outerHTML)', awaitPromise: true, returnByValue: true }, sessionId);
    if (answer.exceptionDetails || typeof answer.result.value !== 'string') throw new Error('Chrome did not return the font-ready document');
    return answer.result.value;
  } finally {
    stopping = true;
    // The isolated browser serves only this fixture; stop its held process after reading the result.
    if (child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
    await closed;
  }
}

async function evidence(name: string, width: number, selector: string) {
  const subject = element(selector);
  const rect = subject.getBoundingClientRect();
  console.info(JSON.stringify({ name, viewport: width, layoutMeasured: rect.width > 0, width: rect.width, clientWidth: subject.clientWidth, scrollWidth: subject.scrollWidth }));
  const directory = process.env.LYSFIX_UI_LAYOUT_DIR;
  const browser = process.env.LYSFIX_UI_LAYOUT_BROWSER;
  if (!directory) {
    if (browser) throw new Error('LYSFIX_UI_LAYOUT_DIR must name the fixture directory for browser layout checks');
    return;
  }
  const compiledCss = process.env.LYSFIX_UI_LAYOUT_CSS;
  if (!compiledCss) throw new Error('LYSFIX_UI_LAYOUT_CSS must name the built stylesheet when exporting browser fixtures');
  const body = document.body.cloneNode(true) as HTMLElement;
  document.querySelectorAll('input').forEach((input, index) => {
    const copy = body.querySelectorAll('input')[index];
    copy.value = input.value;
    copy.setAttribute('value', input.value);
    copy.toggleAttribute('checked', input.checked);
  });
  document.querySelectorAll('option').forEach((option, index) => body.querySelectorAll('option')[index].toggleAttribute('selected', option.selected));
  const measure = `addEventListener('load', async () => {
    await document.fonts.ready;
    const target = document.querySelector(${JSON.stringify(selector)});
    const measure = (element) => {
      const rect = element.getBoundingClientRect(), css = getComputedStyle(element);
      return { tag: element.tagName, text: element.textContent, width: rect.width, left: rect.left, right: rect.right, clientWidth: element.clientWidth, scrollWidth: element.scrollWidth, whiteSpace: css.whiteSpace, overflowX: css.overflowX };
    };
    const result = { name: ${JSON.stringify(name)}, viewport: innerWidth, target: measure(target), parent: measure(target.parentElement), labels: [...target.querySelectorAll('label, legend')].map(measure) };
    const proof = document.createElement('pre'); proof.id = 'layout-proof'; proof.textContent = JSON.stringify(result); document.body.appendChild(proof);
  });`;
  const fixture = join(directory, name + '-' + width + '.html');
  writeFileSync(fixture, '<!doctype html><html><head><meta name="viewport" content="width=device-width, initial-scale=1"><style>' + readFileSync(compiledCss, 'utf8') + '</style></head><body>' + body.innerHTML + '<script>' + measure + '</script></body></html>');
  if (!browser) return;
  const profile = mkdtempSync(join(tmpdir(), 'lysfix-ui-browser-'));
  try {
    const html = await browserDocument(browser, fixture, width, profile);
    const page = new DOMParser().parseFromString(html, 'text/html');
    const measured = page.getElementById('layout-proof')?.textContent;
    if (!measured) throw new Error('Chrome did not emit the load/font-ready layout event for ' + name);
    type Box = { width: number; left: number; right: number; clientWidth: number; scrollWidth: number; whiteSpace: string; overflowX: string };
    const proof = JSON.parse(measured) as { viewport: number; target: Box; parent: Box; labels: Box[] };
    writeFileSync(join(directory, name + '-' + width + '.json'), measured);
    const dimensions = (box: Box) => ({ width: box.width, left: box.left, right: box.right, clientWidth: box.clientWidth, scrollWidth: box.scrollWidth, whiteSpace: box.whiteSpace, overflowX: box.overflowX });
    console.info(JSON.stringify({ name, viewport: proof.viewport, layoutMeasured: true, target: dimensions(proof.target), parent: dimensions(proof.parent), labels: proof.labels.map(dimensions) }));
    expect(proof.viewport).toBe(width);
    expect(proof.target.width).toBeGreaterThan(0);
    expect(proof.target.scrollWidth).toBeLessThanOrEqual(proof.target.clientWidth + 1);
    expect(proof.target.left).toBeGreaterThanOrEqual(proof.parent.left - 1);
    expect(proof.target.right).toBeLessThanOrEqual(proof.parent.right + 1);
    expect(['auto', 'scroll']).not.toContain(proof.target.overflowX);
    expect(proof.labels).toHaveLength(name === 'builder' ? 2 : 0);
    if (name === 'give') expect(['nowrap', 'pre']).not.toContain(proof.parent.whiteSpace);
    for (const label of proof.labels) {
      expect(['nowrap', 'pre']).not.toContain(label.whiteSpace);
      expect(label.scrollWidth).toBeLessThanOrEqual(label.clientWidth + 1);
      expect(label.right).toBeLessThanOrEqual(proof.target.right + 1);
    }
  } finally {
    rmSync(profile, { recursive: true });
  }
}

it.each([1440, 1024])('drives the Give chooser with a long resource label at %i pixels without authored nowrap', async (width) => {
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: width });
  location.hash = '#/file/' + SCRIBE + '/access';
  await act(async () => {
    root.render(<App />);
  });
  await click('[data-act="grant"]');
  const chooser = element<HTMLSelectElement>('[aria-label="Grant to give"]');
  expect(chooser.value).toBe('');
  expect(chooser.options).toHaveLength(3);
  await act(async () => {
    chooser.value = second.id;
    chooser.dispatchEvent(new Event('change', { bubbles: true }));
  });
  expect(chooser.selectedOptions[0]?.textContent).toContain(resource);
  expect(element<HTMLInputElement>('input[name="action"]').checked).toBe(false);
  await click('input[name="action"]');
  expect(element<HTMLInputElement>('input[name="action"]').checked).toBe(true);
  expect(getComputedStyle(chooser).whiteSpace).not.toBe('nowrap');
  expect(getComputedStyle(chooser.parentElement as HTMLElement).whiteSpace).not.toBe('nowrap');
  await evidence('give', width, '[aria-label="Grant to give"]');
});

it.each([1440, 1024])('drives the agent action fieldset with a long action at %i pixels without authored nowrap', async (width) => {
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: width });
  location.hash = '#/apps';
  await act(async () => {
    root.render(<App />);
  });
  await click('[aria-label="Edit the permission template"]');
  const fieldset = element<HTMLFieldSetElement>('[aria-label="Agent actions on record"]');
  expect(fieldset.querySelector('legend')?.textContent).toBe('Actions agents may hold');
  const choice = element<HTMLInputElement>('[aria-label="Agents may hold ' + action + ' on record"]');
  expect(choice.checked).toBe(true);
  await act(async () => {
    choice.click();
  });
  expect(choice.checked).toBe(false);
  await act(async () => {
    choice.click();
  });
  expect(choice.checked).toBe(true);
  expect(getComputedStyle(fieldset).flexWrap).toBe('wrap');
  for (const label of fieldset.querySelectorAll('label')) expect(getComputedStyle(label).whiteSpace).not.toBe('nowrap');
  await evidence('builder', width, '[aria-label="Agent actions on record"]');
});

it('wraps every action name only at shared identifier boundaries and keeps layout in the stylesheet', async () => {
  const identifier = 'read_clinical.records-for:assigned/wards';
  const record = { name: 'record', actions: [identifier], relations: [], parents: [], agents: [identifier] };
  const draft = { kinds: [record] };
  await act(async () => {
    root.render(<KindNode app="cambium" draft={draft} kind={record} seen={[]} change={(edit) => edit(draft)} />);
  });
  const fieldset = element<HTMLFieldSetElement>('[aria-label="Agent actions on record"]');
  const label = fieldset.querySelector('label');
  if (!label) throw new Error('The agent action label was not rendered');
  const name = label.querySelector('span');
  if (!name) throw new Error('The agent action name was not rendered');
  expect(name.querySelectorAll('wbr')).toHaveLength(5);
  const names = document.querySelectorAll<HTMLElement>('.sb-action-name');
  expect(names).toHaveLength(3);
  for (const shown of names) {
    expect(shown.textContent).toBe(identifier);
    expect([...shown.childNodes].filter((node) => node.nodeType === Node.TEXT_NODE).map((node) => node.textContent)).toEqual(['read_', 'clinical.', 'records-', 'for:', 'assigned/', 'wards']);
    expect(shown.querySelectorAll('wbr')).toHaveLength(5);
    const css = getComputedStyle(shown);
    expect(css.whiteSpace).toBe('normal');
    expect(css.overflowWrap).toBe('normal');
    expect(css.wordBreak).toBe('normal');
    expect(css.hyphens).toBe('none');
  }
  expect(document.querySelector('.sb-node')?.querySelectorAll('[style]')).toHaveLength(0);
  expect(Number.parseFloat(getComputedStyle(fieldset).minWidth)).toBe(0);
  expect(getComputedStyle(fieldset).maxWidth).toBe('100%');
});
