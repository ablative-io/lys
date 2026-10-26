import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent, ReactNode } from 'react';
import { useNavigate } from 'react-router';
import { api, request } from '../api';
import { signOutNow } from '../features/me/SignOut';
import { visibleEntries } from '../features/people/directory';
import type { Entry } from '../features/people/directory';
import { keyable } from './keyable';
import { useShell } from './ShellContext';
import { GO } from './keys';
import { RAIL } from './railItems';
import { useNavigationAccess } from './NavigationAccess';

interface Item {
  g: string;
  t: string;
  /** What is shown at the right: a key that does the same, or a short description; never an address. */
  d: string;
  /** Where a Go to row leads, kept on the row for the keyboard and tests, never shown. */
  to?: string;
  go: () => void;
}

/** The g key that reaches an address, as the key registry has it: "g p". */
const keyFor = (hash: string): string => {
  const letter = Object.entries(GO).find(([, path]) => '#' + path === hash)?.[0];
  return letter ? 'g ' + letter : '';
};

export const PAGES: [string, string][] = [
  ['Dashboard', '#/dashboard'], ['Waiting on you', '#/?section=waiting-on-you'], ['Requests', '#/requests'], ['People and agents', '#/people'], ['Running agents', '#/canvas'],
  ['Access', '#/access'], ['Access: ask', '#/access/can'], ['Access: drafts', '#/access/drafts'], ['Access: reviews', '#/reviews'],
  ['Access: resources', '#/resources'], ['Access: model', '#/model'], ['Access map', '#/graph'], ['Jobs', '#/roles'],
  ['Computers', '#/network'], ['Apps', '#/apps'], ['Secrets', '#/secrets'], ['Settings', '#/settings/account'], ['Settings: this service', '#/settings'], ['Settings: service accounts', '#/settings/service-accounts'], ['Settings: services and sign-in providers', '#/connections'], ['You', '#/me'],
];

/** The rail's own pages, read from the rail itself: what Search offers before anything is typed. Home's address is #/, which Search names #/dashboard. */
const ON_RAIL = new Set(RAIL.flatMap((item) => item.t === 'a' && item.href ? [item.href === '#/' ? '#/dashboard' : item.href] : []));

export function Palette() {
  const shell = useShell();
  const mayOpen = useNavigationAccess();
  const navigate = useNavigate();
  const [query, setQuery] = useState('');
  const [sel, setSel] = useState(0);
  const [ids, setIds] = useState<Entry[]>([]);
  // The agents running now, so a name typed here goes straight to that agent's terminal on the canvas.
  const [running, setRunning] = useState<{ agent: string | null; machine: string; machine_name?: string | null }[]>([]);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const open = shell.paletteOpen;

  useEffect(() => {
    if (!open) return;
    setQuery('');
    setSel(0);
    input.current?.focus();
    let live = true;
    api.people().then(
      (answer) => live && setIds(visibleEntries(answer)),
      () => live && setIds([]),
    );
    request<{ sessions: typeof running }>('/runtime/live').then(
      (answer) => live && setRunning(Array.isArray(answer.sessions) ? answer.sessions : []),
      () => live && setRunning([]),
    );
    return () => {
      live = false;
    };
  }, [open]);

  const go = (hash: string) => navigate(hash.slice(1));
  const q = query.toLowerCase();
  const typed = q.trim() !== '';
  // Nothing typed: what can be done (the rows only Search has), then the rail's pages in its order, the same rows in the same place every time.
  // Typed: every page and view, then people, running agents and the reach question, so a long directory never pushes the pages down.
  const pages: Item[] = PAGES.filter(([, h]) => mayOpen(h.slice(1)) && (typed || ON_RAIL.has(h))).map(([t, h]) => ({ g: 'Go to', t, d: keyFor(h), to: h, go: () => go(h) }));
  const acts: Item[] = [
    ...([['Add a person', '#/people/new'], ['Add an agent', '#/agents/new']] as const)
      .filter(([, h]) => mayOpen(h.slice(1))).map(([t, h]) => ({ g: 'Do', t, d: '', to: h, go: () => go(h) })),
    { g: 'Do', t: 'Move the menu and Help to the other side', d: '\\', go: shell.toggleDockSide },
    { g: 'Do', t: 'Show or hide menu labels', d: '[', go: shell.toggleLabels },
    // Ends this browser's session only; a failure stays on screen as a toast until dismissed.
    { g: 'Do', t: 'Sign out', d: 'this browser', go: () => { void signOutNow().then((problem) => { if (problem) shell.toast(problem); }); } },
  ];
  const found: Item[] = typed ? [
    ...ids.map((x) => ({ g: 'People and agents', t: x.display_name, d: `${x.role ?? x.kind} · ${x.state}`, go: () => go('#/file/' + x.id) })),
    ...running.flatMap((session) => {
      const agent = ids.find((x) => x.id === session.agent);
      const to = '#/canvas/' + encodeURIComponent(session.agent ?? '');
      return agent ? [{ g: 'Running now', t: agent.display_name + ': terminal', d: 'on ' + (session.machine_name ?? session.machine), to, go: () => go(to) }] : [];
    }),
    ...ids.map((x) => ({ g: 'Ask', t: `What can ${x.display_name} reach?`, d: '', go: () => go('#/access/reach/' + x.id) })),
  ] : [];
  const source: Item[] = [...acts, ...pages, ...found];
  const items = source.filter((i) => (i.t + ' ' + i.d + ' ' + i.g).toLowerCase().includes(q));
  const at = Math.min(sel, Math.max(0, items.length - 1));

  useEffect(() => {
    list.current?.querySelector('.sel')?.scrollIntoView?.({ block: 'nearest' });
  }, [at]);

  const choose = (item: Item | undefined) => {
    if (!item) return;
    shell.closeAll();
    item.go();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSel(Math.min(at + 1, items.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSel(Math.max(at - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      choose(items[at]);
    }
  };

  // While open, each row is reachable by Tab from the input and answers Enter and
  // Space for itself, never for the row the arrows mark; closed, it is out of the
  // Tab order, as the input is.
  let group = '';
  const rows: ReactNode[] = [];
  items.forEach((item, n) => {
    if (item.g !== group) {
      group = item.g;
      rows.push(<div className="grp" key={'g' + n}>{group}</div>);
    }
    rows.push(
      <div className={'it' + (n === at ? ' sel' : '')} data-n={n} data-to={item.to} key={n} onClick={() => choose(item)} {...(open ? keyable(() => choose(item)) : {})}>
        <span>{item.t}</span>
        <span className="dim mono">{item.d}</span>
      </div>,
    );
  });

  return (
    <div className={'palette' + (open ? ' open' : '')} id="palette" role="dialog" aria-label="Search" aria-hidden={!open}>
      <input
        id="palIn"
        ref={input}
        placeholder="Find a page, a person or an agent, or something to do…"
        autoComplete="off"
        tabIndex={open ? 0 : -1}
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
          setSel(0);
        }}
        onKeyDown={onKey}
      />
      <div className="list" id="palList" ref={list}>
        {rows.length ? rows : <div className="it dim">Nothing by that name.</div>}
      </div>
    </div>
  );
}
