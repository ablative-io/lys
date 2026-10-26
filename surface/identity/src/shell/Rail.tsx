import { useEffect, useState } from 'react';
import { useLocation } from 'react-router';
import { api, Refused, useLoad } from '../api';
import { refreshLive } from '../live';
import { focusWaiting, railWaiting, watchWaiting } from '../features/dashboard/waiting';
import { SignOut } from '../features/me/SignOut';
import type { MeView } from '../generated';
import { Act } from './Act';
import { RAIL } from './railItems';
import { useShell } from './ShellContext';
import { useNavigationVerdict } from './NavigationAccess';

/** The view a hash path belongs to, as the rail marks it. */
export function railView(pathname: string): string {
  const view = (pathname.split('/')[1] || 'dashboard').replace(/^team$/, 'dashboard');
  if (view === 'file' || view === 'agents' || view === 'service-accounts') return 'people';
  if (view === 'requests' || view === 'reviews') return 'waiting';
  if (['resources', 'model', 'graph', 'roles'].includes(view)) return 'access';
  if (view === 'connections') return 'settings';
  return view;
}

/** ⌘ on a Mac, Ctrl elsewhere: the key a person on this computer actually presses. */
const onMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
const forThisComputer = (keys: string): string => onMac ? keys : keys.replace('⌘', 'Ctrl ');

type Waiting = { count: number; words: string } | { problem: string } | null;
const waitingOf = (refused: Refused): Waiting => refused.status === 401 || refused.status === 403 ? null : { problem: refused.refusal.reason };

/**
 * Who is signed in, and what waits on them: `GET /waiting`'s one list, the same Home's Waiting on you draws, so the
 * number here is the number of lines there. A part the service could not read is named, never counted as nothing.
 * The person stays known when the waiting read is refused, so Sign out is drawn for everyone.
 */
function useWaiting(): { me: MeView | null; waiting: Waiting } {
  const load = useLoad(api.me, 'rail-person');
  const me = load.status === 'ok' ? load.data : null;
  const [waiting, setWaiting] = useState<Waiting>(null);
  useEffect(() => {
    if (!me) return undefined;
    return watchWaiting((lines) => setWaiting(railWaiting(lines)), (error: unknown) => {
      setWaiting(waitingOf(error instanceof Refused ? error : new Refused(0, { refusal: 'WaitingUnreadable', reason: String(error) })));
    });
  }, [me]);
  if (load.status === 'refused') return { me: null, waiting: waitingOf(load.refused) };
  return { me, waiting };
}

export function Rail() {
  const shell = useShell();
  const verdictOf = useNavigationVerdict();
  const { me, waiting } = useWaiting();
  const { pathname } = useLocation();
  const on = railView(pathname);
  // On a phone the rail is a bar with a Menu button; the menu opens the same rail over the page.
  const [drawer, setDrawer] = useState(false);
  useEffect(() => setDrawer(false), [pathname]);
  useEffect(() => {
    if (!drawer) return undefined;
    const shut = (event: KeyboardEvent) => { if (event.key === 'Escape') setDrawer(false); };
    document.addEventListener('keydown', shut);
    return () => document.removeEventListener('keydown', shut);
  }, [drawer]);
  const count = waiting && ('problem' in waiting ? true : waiting.count > 0)
    ? <span className="cnt" title={'problem' in waiting ? 'What waits on you could not be read: ' + waiting.problem : waiting.words}>{'problem' in waiting ? '?' : waiting.count}</span>
    : null;
  const press = (id: string | undefined, dock: 'help' | undefined) => {
    setDrawer(false);
    // Help and ? are one thing: this screen, explained where it stands; its foot opens every topic.
    if (dock) return shell.explaining ? shell.explainOff() : shell.explainOn();
    if (id === 'palBtn') return shell.openPalette();
    if (id === 'railBtn') return shell.toggleLabels();
    return undefined;
  };
  return (
    <nav className={'rail' + (shell.labels || drawer ? ' open' : '') + (drawer ? ' drawer' : '')} id="rail" aria-label="Main">
      <div className="mark">
        <span className="seal">L</span>
        <span className="lbl">Lys</span>
      </div>
      <div className="menu">
        {drawer ? null : count}
        <Act className="quiet menu-btn" symbol={drawer ? 'close' : 'menu'} name={drawer ? 'Close the menu' : 'Open the menu'} word={drawer ? 'Close' : 'Menu'}
          aria-expanded={drawer} onClick={() => setDrawer(!drawer)} />
      </div>
      {RAIL.map((item, index) => {
        if (item.t === 'sep') return <div className="sep" key={index} />;
        if (item.t === 'grow') return <div className="grow" key={index} />;
        // Every entry is drawn from the first frame, so the rail never moves; one the service would not open is muted and says why.
        const verdict = item.t === 'a' && item.href ? verdictOf(item.href.slice(1)) : { open: true as const };
        const inner = (
          <>
            <svg viewBox="0 0 24 24" dangerouslySetInnerHTML={{ __html: item.svg }} />
            <span className="lbl">{item.label}</span>
            {item.nav === 'waiting' ? count : null}
            {item.kbd ? <span className="kbd lbl">{forThisComputer(item.kbd)}</span> : null}
          </>
        );
        if (item.t === 'a') {
          const link = (
            <a key={index} href={item.href} data-nav={item.nav} data-shut={verdict.open ? undefined : verdict.definite ? 'refused' : 'unchecked'}
              onClick={() => { setDrawer(false); if (item.nav === 'waiting') { focusWaiting(); refreshLive(); } }}
              title={forThisComputer(item.title) + (verdict.open ? '' : ': ' + verdict.words)} className={item.nav === on ? 'on' : undefined}>
              {inner}
            </a>
          );
          // Sign out sits under You, with the signed-in person's name; with labels off the name hides with the rest.
          if (item.nav !== 'me' || !me) return link;
          return [link, <SignOut key="sign-out" place="rail" who={me.person.display_name} person={me.person.id} told={shell.toast} />];
        }
        return (
          <button
            key={index}
            className={'rb' + (item.dock && shell.explaining ? ' on' : '')}
            data-dockbtn={item.dock}
            id={item.id}
            title={forThisComputer(item.title)}
            onClick={() => press(item.id, item.dock)}
          >
            {inner}
          </button>
        );
      })}
    </nav>
  );
}
