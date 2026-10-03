import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent, ReactNode } from 'react';
import { useNavigate } from 'react-router';
import { api } from '../api';
import { entries } from '../features/people/directory';
import type { Entry } from '../features/people/directory';
import { keyable } from './keyable';
import { useShell } from './ShellContext';

interface Item {
  g: string;
  t: string;
  d: string;
  go: () => void;
}

export const PAGES: [string, string][] = [
  ['People and agents', '#/people'], ['Roles', '#/roles'], ['Resources', '#/resources'], ['Access', '#/access'],
  ['Requests', '#/requests'], ['Reviews', '#/reviews'], ['Credentials', '#/vault'], ['Connections', '#/connections'],
  ['Sign-ins', '#/sessions'], ['Model', '#/model'], ['Try a change', '#/model/try'], ['Configuration', '#/settings'],
  ['Graph', '#/graph'], ['Secrets', '#/secrets'], ['Computers', '#/network'], ['Running', '#/runtime'], ['You', '#/me'], ['Agent canvas', '#/canvas'],
];

export function Palette() {
  const shell = useShell();
  const navigate = useNavigate();
  const [query, setQuery] = useState('');
  const [sel, setSel] = useState(0);
  const [ids, setIds] = useState<Entry[]>([]);
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
      (answer) => live && setIds(entries(answer)),
      () => live && setIds([]),
    );
    return () => {
      live = false;
    };
  }, [open]);

  const go = (hash: string) => navigate(hash.slice(1));
  const source: Item[] = [
    ...ids.map((x) => ({ g: 'People and agents', t: x.display_name, d: `${x.role ?? x.kind} · ${x.state}`, go: () => go('#/file/' + x.id) })),
    ...ids.map((x) => ({ g: 'Ask', t: `What can ${x.display_name} reach?`, d: '', go: () => go('#/access/reach/' + x.id) })),
    { g: 'Acts', t: 'Add an agent', d: '', go: () => go('#/agents/new') },
    { g: 'Acts', t: 'Toggle dock side', d: '\\', go: shell.toggleDockSide },
    { g: 'Acts', t: 'Toggle rail labels', d: '[', go: shell.toggleLabels },
    ...PAGES.map(([t, h]) => ({ g: 'Go to', t, d: h, go: () => go(h) })),
  ];
  const q = query.toLowerCase();
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
      <div className={'it' + (n === at ? ' sel' : '')} data-n={n} key={n} onClick={() => choose(item)} {...(open ? keyable(() => choose(item)) : {})}>
        <span>{item.t}</span>
        <span className="dim mono">{item.d}</span>
      </div>,
    );
  });

  return (
    <div className={'palette' + (open ? ' open' : '')} id="palette" role="dialog" aria-label="Command palette" aria-hidden={!open}>
      <input
        id="palIn"
        ref={input}
        placeholder="Find a person, an agent, a resource, or an act…"
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
