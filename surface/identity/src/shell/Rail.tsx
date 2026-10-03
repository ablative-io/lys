import { useLocation } from 'react-router';
import { RAIL } from './railItems';
import { useShell } from './ShellContext';

/** The view a hash path belongs to, as the rail marks it. */
export function railView(pathname: string): string {
  const view = (pathname.split('/')[1] || 'people').replace(/^team$/, 'people');
  if (view === 'file') return 'people';
  if (view === 'vault') return 'secrets';
  return view;
}

export function Rail() {
  const shell = useShell();
  const { pathname } = useLocation();
  const on = railView(pathname);
  const press = (id: string | undefined, dock: 'help' | 'assistant' | undefined) => {
    if (dock) return shell.toggleDock(dock);
    if (id === 'palBtn') return shell.openPalette();
    if (id === 'railBtn') return shell.toggleLabels();
    return undefined;
  };
  return (
    <nav className={'rail' + (shell.labels ? ' open' : '')} id="rail" aria-label="Main">
      <div className="mark">
        <span className="seal">ID</span>
        <span className="lbl">
          Identity <span className="open-q">name open</span>
        </span>
      </div>
      {RAIL.map((item, index) => {
        if (item.t === 'sep') return <div className="sep" key={index} />;
        if (item.t === 'grow') return <div className="grow" key={index} />;
        const inner = (
          <>
            <svg viewBox="0 0 24 24" dangerouslySetInnerHTML={{ __html: item.svg }} />
            <span className="lbl">{item.label}</span>
            {item.cnt ? <span className="cnt" id={item.cnt} style={{ display: 'none' }} /> : null}
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
