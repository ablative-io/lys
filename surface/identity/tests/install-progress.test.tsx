/** Lys.app's progress page: every step as it happens, the engine's guidance, a failure's next step, the hand-over. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from '../src/App';
import { EVENTS, InstallProgress, RETRY } from '../src/features/install/InstallProgress';
import type { InstallView, StepView } from '../src/features/install/InstallProgress';
import { $, $$, click, settle, text } from './harness';

/** An EventSource the test speaks through, as the app's server would. */
class FakeSource {
  static readonly CLOSED = 2;
  static made: FakeSource[] = [];
  readonly url: string;
  readyState = 1;
  onmessage: ((event: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  constructor(url: string) {
    this.url = url;
    FakeSource.made.push(this);
  }
  close() {
    this.readyState = FakeSource.CLOSED;
  }
  async send(view: InstallView) {
    await act(async () => this.onmessage?.({ data: JSON.stringify(view) }));
  }
}

const WORDS = [
  ['engine', 'Checking the container engine'],
  ['directory', 'Preparing your directory'],
  ['sign_in', 'Starting sign-in'],
  ['clients', 'Connecting sign-in to Lys'],
  ['keys', 'Making Lys’s keys'],
  ['screens', 'Placing Lys’s screens'],
  ['start', 'Starting Lys'],
  ['build', 'Recording what is installed'],
];

function steps(now: number): StepView[] {
  return WORDS.map(([name, words], index) => ({
    name, words, state: index < now ? 'done' : index === now ? 'now' : 'waiting',
  }));
}

let version = 1;
const working = (now: number): InstallView => ({
  version: ++version, note: null, phase: 'working', work: 'install', title: 'Installing Lys', steps: steps(now), said: [],
});

let root: Root | null = null;

async function open(go: (url: string) => void = () => {}): Promise<FakeSource> {
  FakeSource.made = [];
  vi.stubGlobal('EventSource', FakeSource);
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => root?.render(<InstallProgress go={go} />));
  const [source] = FakeSource.made;
  if (!source) throw new Error('the page opened no event stream');
  return source;
}

afterEach(() => {
  act(() => root?.unmount());
  root = null;
});

describe('Install progress', () => {
  it('follows the app’s own event stream and shows each step as it happens', async () => {
    const source = await open();
    expect(source.url).toBe(EVENTS);
    expect($('h1')?.textContent).toBe('Opening Lys');
    await source.send(working(1));
    expect($('h1')?.textContent).toBe('Installing Lys');
    expect($('[data-state="now"]')?.textContent).toBe('Preparing your directory');
    await source.send(working(2));
    expect($('[data-state="now"]')?.textContent).toBe('Starting sign-in');
    expect($$('[data-state="done"]').map((step) => step.dataset.step)).toEqual(['engine', 'directory']);
    expect($$('[data-step]')).toHaveLength(8);
  });

  it('guides a missing engine to its download and continues by itself', async () => {
    const source = await open();
    await source.send({
      version: ++version, note: null, phase: 'engine',
      guidance: {
        state: 'missing', title: 'Lys needs a container engine', why: 'Lys keeps its sign-in in a container engine.',
        what: 'Download Docker Desktop for a Mac with an Apple chip.', download: 'https://desktop.docker.com/mac/main/arm64/Docker.dmg',
        page: 'https://docs.docker.com/desktop/setup/install/mac-install/', then: 'Lys continues by itself as soon as the engine answers.',
      },
    });
    expect($('#engine-guidance')?.dataset.engine).toBe('missing');
    expect($('#engine-guidance a.primary')?.getAttribute('href')).toBe('https://desktop.docker.com/mac/main/arm64/Docker.dmg');
    expect(text()).toContain('continues by itself');
    expect($$('button')).toHaveLength(0);
    await source.send(working(2));
    expect($('#engine-guidance')).toBeNull();
    expect($('[data-state="now"]')?.textContent).toBe('Starting sign-in');
  });

  it('shows a failure in its words with the one next thing to do, and Try again resumes', async () => {
    const posted: string[] = [];
    const source = await open();
    vi.stubGlobal('fetch', async (input: string, init?: RequestInit) => {
      posted.push(`${init?.method ?? 'GET'} ${input}`);
      return new Response('{"retry":true}', { status: 202 });
    });
    await source.send({
      version: ++version, note: null, phase: 'failed', refusal: 'install_step_failed',
      words: 'Starting sign-in did not finish.', next: 'Press Try again. Lys picks up where it stopped.', retry: true, steps: steps(2),
    });
    expect($('[role="alert"]')?.dataset.refusal).toBe('install_step_failed');
    expect(text()).toContain('Starting sign-in did not finish.');
    expect(text()).toContain('Press Try again.');
    expect(source.readyState).not.toBe(FakeSource.CLOSED);
    await click($('.why-not button'));
    expect(posted).toEqual([`POST ${RETRY}`]);
    await source.send(working(2));
    expect($('.why-not')).toBeNull();
  });

  it('keeps a refusal Try again cannot help, with no button, and stops listening', async () => {
    const source = await open();
    await source.send({
      version: ++version, note: null, phase: 'failed', refusal: 'app_not_in_applications',
      words: 'Lys is running from the disk image, not from your Applications folder.',
      next: 'Quit Lys, drag it to Applications, then open it from there.', retry: false, steps: [],
    });
    expect(text()).toContain('drag it to Applications');
    expect($$('button')).toHaveLength(0);
    expect(source.readyState).toBe(FakeSource.CLOSED);
  });

  it('hands the browser to first-run setup when Lys is ready', async () => {
    const went: string[] = [];
    const source = await open((url) => went.push(url));
    await source.send({ version: ++version, note: 'Lys was uninstalled. Your data folder was kept.', phase: 'ready', words: 'Lys is ready. Taking you there now.', url: 'http://localhost:8490/setup' });
    expect(went).toEqual(['http://localhost:8490/setup']);
    expect(source.readyState).toBe(FakeSource.CLOSED);
    expect($('#last-uninstall')?.textContent).toContain('data folder was kept');
    expect($('[role="status"] a')?.getAttribute('href')).toBe('http://localhost:8490/setup');
  });

  it('names what it could not read, never showing an empty page', async () => {
    const source = await open();
    await act(async () => source.onmessage?.({ data: '{"phase":"elsewhere"}' }));
    expect($('#install-lost')?.textContent).toBe('Lys sent this page something it could not read.');
    await act(async () => source.onerror?.());
    expect($('#install-lost')?.textContent).toContain('reconnects by itself');
  });

  it('never names the issuer, a port or a terminal on any phase', async () => {
    const source = await open();
    const seen: string[] = [];
    for (const view of [working(0), working(3), working(7)]) {
      await source.send(view);
      seen.push(text());
    }
    for (const page of seen) {
      expect(page.toLowerCase()).not.toMatch(/rauthy|terminal|127\.0\.0\.1|password/);
    }
    expect(seen).toHaveLength(3);
  });

  it('is what the app shows at /install, with no sign-in', async () => {
    FakeSource.made = [];
    vi.stubGlobal('EventSource', FakeSource);
    history.replaceState(null, '', '/install');
    const container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    await act(async () => root?.render(<App />));
    await settle();
    expect(FakeSource.made.map((source) => source.url)).toEqual([EVENTS]);
    expect($('.install-progress h1')?.textContent).toBe('Opening Lys');
  });
});
