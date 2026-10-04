import { useLocation } from 'react-router';
import { api, request, useLoad } from '../api';
import { readTogether } from '../reads';
import { RAIL } from './railItems';
import { useShell } from './ShellContext';

/** The view a hash path belongs to, as the rail marks it. */
export function railView(pathname: string): string {
  const view = (pathname.split('/')[1] || 'me').replace(/^team$/, 'me');
  if (view === 'file') return 'people';
  if (['resources', 'requests', 'reviews', 'model'].includes(view)) return 'access';
  if (view === 'connections' || view === 'apps') return 'settings';
  return view;
}

/** What waits under Access: requests awaiting a decision and grants due a review, as the service lists them for whoever is signed in. */
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
        const inner = (
          <>
            <svg viewBox="0 0 24 24" dangerouslySetInnerHTML={{ __html: item.svg }} />
            <span className="lbl">{item.label}</span>
            {item.nav === 'access' && waiting && ('problem' in waiting ? true : waiting.count > 0) ? <span className="cnt" title={'problem' in waiting ? 'What waits under Access could not be read: ' + waiting.problem : waiting.words}>{'problem' in waiting ? '?' : waiting.count}</span> : null}
            {item.kbd ? <span className="kbd lbl">{item.kbd}</span> : null}
          </>
        );
        if (item.t === 'a') {
          return (
            <a key={index} href={item.href} data-nav={item.nav} title={item.title} className={item.nav === on ? 'on' : undefined}>
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
            title={item.title}
            onClick={() => press(item.id, item.dock)}
          >
            {inner}
          </button>
        );
      })}
    </nav>
  );
}
