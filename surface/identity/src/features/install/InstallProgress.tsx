/**
 * Lys.app's progress page, served by the app itself on a loopback port at /install.
 *
 * The app sends each change as a server-sent event on /install/events: the steps of the
 * install as they begin, the container engine's guidance while the app waits for it, a
 * failure's words with the one next thing to do, and, when Lys is ready, the address the
 * browser is handed to. The page never asks again on a schedule; it is told. It shows only
 * the plain words the app sends, which never name a program, a port, a password file or the
 * issuer inside Lys.
 */
import { useEffect, useState } from 'react';

export interface StepView { name: string; words: string; state: 'done' | 'now' | 'waiting' }

export interface Guidance {
  state: 'missing' | 'stopped';
  title: string;
  why: string;
  what: string;
  download: string | null;
  page: string;
  then: string;
}

export type Phase =
  | { phase: 'opening' }
  | { phase: 'working'; work: string; title: string; steps: StepView[]; said: string[] }
  | { phase: 'engine'; guidance: Guidance }
  | { phase: 'failed'; refusal: string; words: string; next: string; retry: boolean; steps: StepView[] }
  | { phase: 'ready'; words: string; url: string };

export type InstallView = Phase & { version: number; note: string | null };

/** Where the app sends each change. */
export const EVENTS = '/install/events';
/** Where "Try again" is sent. */
export const RETRY = '/install/retry';

const PHASES = ['opening', 'working', 'engine', 'failed', 'ready'];

/** One event's data as a view, or an error naming why it is not one. */
export function readView(data: string): InstallView {
  const value: unknown = JSON.parse(data);
  if (!value || typeof value !== 'object' || !('phase' in value) || typeof value.phase !== 'string'
    || !PHASES.includes(value.phase) || !('version' in value) || typeof value.version !== 'number') {
    throw new Error('Lys sent this page something it could not read.');
  }
  return value as InstallView;
}

/** Hands the browser to `url`, in place of this page. */
const leave = (url: string) => location.replace(url);

function Steps({ steps }: { steps: StepView[] }) {
  if (!steps.length) return null;
  return (
    <ol className="install-steps" aria-label="Steps">
      {steps.map((step) => (
        <li key={step.name} data-step={step.name} data-state={step.state} aria-current={step.state === 'now' ? 'step' : undefined}>
          <span className={'dot s-' + (step.state === 'done' ? 'active' : step.state === 'now' ? 'provisioned' : 'retired')} />
          {step.words}
          {step.state === 'done' ? <span className="note"> · done</span> : null}
        </li>
      ))}
    </ol>
  );
}

function EngineGuidance({ guidance }: { guidance: Guidance }) {
  return (
    <div className="card" id="engine-guidance" data-engine={guidance.state}>
      <h2>{guidance.title}</h2>
      <p>{guidance.why}</p>
      <p><b>{guidance.what}</b></p>
      {guidance.download ? (
        <p><a className="btn primary" href={guidance.download}>Download Docker Desktop for this Mac</a></p>
      ) : null}
      <p className="note">More about it on <a href={guidance.page}>Docker’s own page</a>.</p>
      <p role="status">{guidance.then}</p>
    </div>
  );
}

function heading(view: InstallView | null): string {
  if (!view) return 'Opening Lys';
  switch (view.phase) {
    case 'opening': return 'Opening Lys';
    case 'working': return view.title;
    case 'engine': return view.guidance.title;
    case 'failed': return 'Lys stopped';
    case 'ready': return 'Lys is ready';
  }
}

export function InstallProgress({ go = leave }: { go?: (url: string) => void }) {
  const [view, setView] = useState<InstallView | null>(null);
  const [lost, setLost] = useState('');
  const [retrying, setRetrying] = useState(false);

  useEffect(() => {
    if (typeof EventSource === 'undefined') {
      setLost('This browser cannot follow Lys’s progress. Open this page in Safari, Chrome or Firefox.');
      return;
    }
    const source = new EventSource(EVENTS);
    source.onmessage = (event: MessageEvent<string>) => {
      try {
        const next = readView(event.data);
        setView(next);
        setLost('');
        setRetrying(false);
        if (next.phase === 'ready') {
          source.close();
          go(next.url);
        } else if (next.phase === 'failed' && !next.retry) {
          source.close();
        }
      } catch (error) {
        setLost(error instanceof Error ? error.message : String(error));
      }
    };
    source.onerror = () => {
      if (source.readyState === EventSource.CLOSED) {
        setLost('Lys stopped answering this page. Open Lys again from your Applications folder.');
      } else {
        setLost('Lys is not answering this page just now. It reconnects by itself while Lys is open.');
      }
    };
    return () => source.close();
  }, [go]);

  async function retry() {
    setRetrying(true);
    try {
      const answer = await fetch(RETRY, { method: 'POST' });
      if (!answer.ok) throw new Error(`Lys did not take Try again (${answer.status}).`);
    } catch (error) {
      setRetrying(false);
      setLost(error instanceof Error ? error.message : String(error));
    }
  }

  const title = heading(view);
  return (
    <div className="page install-progress" aria-busy={view?.phase === 'working'}>
      <div className="head"><div>
        <div className="eyebrow">Lys</div>
        <h1>{title}</h1>
        {view?.phase === 'working' ? <p className="sub">This takes a few minutes the first time. You can leave this page open.</p> : null}
      </div></div>
      {view?.note ? <p className="card" role="status" id="last-uninstall">{view.note}</p> : null}
      {view?.phase === 'working' ? (
        <>
          <Steps steps={view.steps} />
          {view.said.length ? <ul className="install-said">{view.said.map((line, index) => <li key={index}>{line}</li>)}</ul> : null}
        </>
      ) : null}
      {view?.phase === 'engine' ? <EngineGuidance guidance={view.guidance} /> : null}
      {view?.phase === 'failed' ? (
        <>
          <Steps steps={view.steps} />
          <div role="alert" className="why-not" data-refusal={view.refusal}>
            <b>{view.words}</b>
            <p>{view.next}</p>
            {view.retry ? (
              <button className="btn primary" type="button" onClick={() => void retry()} disabled={retrying}>
                {retrying ? 'Trying again…' : 'Try again'}
              </button>
            ) : null}
          </div>
        </>
      ) : null}
      {view?.phase === 'ready' ? (
        <p role="status">{view.words} <a href={view.url}>Open Lys</a></p>
      ) : null}
      {lost ? <p role="alert" className="note" id="install-lost">{lost}</p> : null}
    </div>
  );
}
