import { useLocation } from 'react-router';
import { api, request, useLoad } from '../api';
import { readTogether } from '../reads';
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

/** What waits on the signed-in person: requests awaiting a decision and grants due a review, as the service lists them. */
function useWaiting(): { count: number; words: string } | { problem: string } | null {
  const load = useLoad(async () => { await api.me(); return readTogether({
    requests: request<{ requests: { state: string }[] }>('/requests'),
    reviews: request<{ due: unknown[] }>('/reviews'),
  }); }, 'rail-waiting');
  if (load.status === 'loading') return null;
  if (load.status === 'refused') return load.refused.status === 401 || load.refused.status === 403 ? null : { problem: load.refused.refusal.reason };
  const waiting = load.data.requests.requests.filter((entry) => entry.state === 'waiting').length;
  const due = load.data.reviews.due.length;
  return { count: waiting + due, words: waiting + (waiting === 1 ? ' request waiting, ' : ' requests waiting, ') + due + (due === 1 ? ' grant to review' : ' grants to review') };
}

export function Rail() {
  const shell = useShell();
  const verdictOf = useNavigationVerdict();
  const waiting = useWaiting();
  const { pathname } = useLocation();
  const on = railView(pathname);
  const press = (id: string | undefined, dock: 'help' | undefined) => {
    if (dock) return shell.toggleDock(dock);
    if (id === 'palBtn') return shell.openPalette();
    if (id === 'railBtn') return shell.toggleLabels();
    return undefined;
  };
  return (
    <nav className={'rail' + (shell.labels ? ' open' : '')} id="rail" aria-label="Main">
      <div className="mark">
        <span className="seal">L</span>
        <span className="lbl">Lys</span>
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
            {item.nav === 'waiting' && waiting && ('problem' in waiting ? true : waiting.count > 0) ? <span className="cnt" title={'problem' in waiting ? 'What waits on you could not be read: ' + waiting.problem : waiting.words}>{'problem' in waiting ? '?' : waiting.count}</span> : null}
            {item.kbd ? <span className="kbd lbl">{forThisComputer(item.kbd)}</span> : null}
          </>
        );
        if (item.t === 'a') {
          return (
            <a key={index} href={item.href} data-nav={item.nav} data-shut={verdict.open ? undefined : verdict.definite ? 'refused' : 'unchecked'}
              title={forThisComputer(item.title) + (verdict.open ? '' : ': ' + verdict.words)} className={item.nav === on ? 'on' : undefined}>
              {inner}
            </a>
          );
        }
        return (
          <button
            key={index}
            className={'rb' + (item.dock && shell.dockMode === item.dock ? ' on' : '')}
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
