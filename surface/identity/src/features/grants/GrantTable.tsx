/** A grant is drawn one way: a row of this table. Giving part of it and revoking it open in a row beneath, in the page. */
import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import type { Grant } from '../../generated/grants';
import type { Column } from '../../shell/Listing';
import { clock } from '../file/time';
import { resourceWords } from './action-words';
import { Delegate } from './Delegate';
import { Chain } from './Chain';
import { chainOf, grantNo, lastUsedText, mayText, nameOf, onText, passText, passesToAgents, voidOf, windowText } from './model';
import type { GrantWorld } from './model';
import { Revoke } from './Revoke';

/** The columns of a grant, the same on every screen that lists one. */
export function grantColumns(w: GrantWorld): Column<Grant>[] {
  return [
    { head: 'Grant', cell: (g) => <span className="mono">{grantNo(g.id)}</span> },
    { head: 'Holder', cell: (g) => nameOf(w, g.holder) },
    { head: 'Allows', cell: (g) => mayText(w, g) },
    { head: 'Relation', cell: (g) => <span className="mono">{g.relation}</span> },
    { head: 'On', cell: (g) => <span title={onText(g)}>{resourceWords(g.resource)}</span> },
    { head: 'Path to a person', cell: (g) => <Chain w={w} chain={chainOf(w, g)} /> },
    { head: 'May pass on', cell: (g) => <span className="sec">{passText(g.pass_on)}</span> },
    { head: 'Window', cell: (g) => <span className="sec">{windowText(g)}</span> },
    { head: 'Last used', cell: (g) => <span className="sec">{g.last_use.seen ? lastUsedText(g) : <span className="dim">{lastUsedText(g)}</span>}</span> },
    { head: 'Stands', cell: (g) => {
      const v = voidOf(w, g);
      if (v === null) return <><span className="dot s-active" />yes</>;
      return <span className="danger"><span className="verdict-mark no" style={{ fontSize: 9, padding: '1px 6px' }}>void</span> {v.why}{g.revoked && g.revoked_at !== null && g.revoked_revision !== null ? ` Revoked ${clock(g.revoked_at)}, change ${g.revoked_revision}; every check from here on refuses.` : ''}</span>;
    } },
  ];
}

type Open = { grant: string; act: 'revoke' | 'delegate'; opener: HTMLElement };

/** The panel an act opens in: it takes the keyboard when it opens, Escape closes it, and the button that opened it, named by the press itself, gets the keyboard back. */
export function ActPanel({ label, opener, close, children }: { label: string; opener: HTMLElement | null; close: () => void; children: ReactNode }) {
  const panel = useRef<HTMLElement>(null);
  useEffect(() => {
    panel.current?.querySelector<HTMLElement>('textarea,input,select,button')?.focus();
    return () => { if (opener?.isConnected) opener.focus(); };
  }, [opener]);
  return <section className="act-panel" aria-label={label} ref={panel} onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); close(); } }}>{children}</section>;
}

export function GrantTable({ w, grants, done, give = true, empty = 'No grants.' }: { w: GrantWorld; grants: Grant[]; done: () => void; give?: boolean; empty?: string }) {
  const [open, setOpen] = useState<Open | null>(null);
  const columns = grantColumns(w);
  const close = () => setOpen(null);
  return <table className="usage-table grant-table" aria-label="Grants">
    <thead><tr>{columns.map((column) => <th key={column.head}>{column.head}</th>)}<th>Change</th></tr></thead>
    <tbody>
      {grants.map((g) => {
        const mayRevoke = !g.revoked && w.who.get(g.holder)?.state !== 'retired';
        const mayGive = give && g.holder === w.me.person.id && g.standing.stands && passesToAgents(g.pass_on);
        const mine = open?.grant === g.id ? open.act : null;
        return [<tr key={g.id} data-grant={g.id} style={voidOf(w, g) === null ? undefined : { opacity: 0.72 }}>
          {columns.map((column) => <td key={column.head} data-col={column.head}>{column.cell(g)}</td>)}

          <td data-col="Change">
            {mayGive ? <button className="btn" type="button" data-act="delegate" data-g={g.id} onClick={(event) => setOpen({ grant: g.id, act: 'delegate', opener: event.currentTarget })}>Give to an agent…</button> : null}
            {mayRevoke ? <button className="btn danger" type="button" data-act="revoke" data-g={g.id} onClick={(event) => setOpen({ grant: g.id, act: 'revoke', opener: event.currentTarget })}>Revoke</button> : null}
          </td>
        </tr>,
        mine && open ? <tr key={g.id + ':' + mine} className="review-row"><td colSpan={columns.length + 1}>
          <ActPanel label={mine === 'revoke' ? 'Revoke' : 'Give'} opener={open.opener} close={close}>{mine === 'revoke' ? <Revoke w={w} g={g} done={done} close={close} /> : <Delegate w={w} source={g} done={done} close={close} />}</ActPanel>
        </td></tr> : null];
      })}
      {grants.length ? null : <tr><td colSpan={columns.length + 1} className="dim">{empty}</td></tr>}
    </tbody>
  </table>;
}
