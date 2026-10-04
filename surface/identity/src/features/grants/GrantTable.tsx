/** A grant is drawn one way: a row of this table. Giving part of it and revoking it open in a row beneath, in the page. */
import { Fragment, useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import type { Grant } from '../../generated/grants';
import type { Column } from '../../shell/Listing';
import { clock } from '../file/time';
import { Delegate } from './Delegate';
import { Chain } from './Chain';
import { chainOf, grantNo, lastUsedText, mayText, nameOf, passText, passesToAgents, resourceName, resourceTitle, voidOf, windowText } from './model';
import type { GrantWorld } from './model';
import { Revoke } from './Revoke';
import './grants.css';

/**
 * An identifier broken only where it has its own breaks (after `.`, `:`, `_`, `-`, `/`),
 * so a long reference wraps between its parts and never inside a word.
 */
export function breakable(text: string): ReactNode {
  const parts = text.match(/[^.:_/-]*[.:_/-]+|[^.:_/-]+$/g) ?? [text];
  return parts.length < 2 ? text : parts.map((part, i) => <Fragment key={i}>{i ? <wbr /> : null}{part}</Fragment>);
}

/** The columns of a grant, the same on every screen that lists one. Grant and Relation never wrap; On and the path take the room. */
export function grantColumns(w: GrantWorld): Column<Grant>[] {
  return [
    { head: 'Grant', cell: (g) => <span className="mono g-tight">{grantNo(g.id)}</span> },
    { head: 'Holder', cell: (g) => nameOf(w, g.holder) },
    { head: 'Allows', cell: (g) => breakable(mayText(w, g)) },
    { head: 'Relation', cell: (g) => <span className="mono g-tight">{g.relation}</span> },
    { head: 'On', cell: (g) => <span className="g-room" title={resourceTitle(w, g.resource)}>{breakable(resourceName(w, g.resource))}</span> },
    { head: 'Path to a person', cell: (g) => <Chain w={w} chain={chainOf(w, g)} /> },
    { head: 'May pass on', cell: (g) => <span className="sec">{passText(g.pass_on)}</span> },
    { head: 'Window', cell: (g) => <span className="sec">{windowText(g)}</span> },
    { head: 'Last used', cell: (g) => <span className="sec">{g.last_use.seen ? lastUsedText(g) : <span className="dim">{lastUsedText(g)}</span>}</span> },
    { head: 'Stands', cell: (g) => {
      const v = voidOf(w, g);
      if (v === null) return <><span className="dot s-active" />yes</>;
      return <span className="danger"><span className="verdict-mark no" style={{ fontSize: 9, padding: '1px 6px' }}>void</span> {breakable(v.why)}{g.revoked && g.revoked_at !== null && g.revoked_revision !== null ? ` Revoked ${clock(g.revoked_at)}, change ${g.revoked_revision}; every check from here on refuses.` : ''}</span>;
    } },
  ];
}

export type Act = 'revoke' | 'delegate';

/**
 * A grant's Change: give part of it to an agent, or revoke it. The same buttons
 * wherever a grant is listed; a press never also opens the row it sits in.
 */
export function ChangeButtons({ w, g, give, open }: { w: GrantWorld; g: Grant; give: boolean; open: (act: Act, opener: HTMLElement) => void }) {
  const mayRevoke = !g.revoked && w.who.get(g.holder)?.state !== 'retired';
  const mayGive = give && g.holder === w.me.person.id && g.standing.stands && passesToAgents(g.pass_on);
  return <span className="g-change">
    {mayGive ? <button className="btn" type="button" data-act="delegate" data-g={g.id} onClick={(event) => { event.stopPropagation(); open('delegate', event.currentTarget); }}>Give to an agent…</button> : null}
    {mayRevoke ? <button className="btn danger" type="button" data-act="revoke" data-g={g.id} onClick={(event) => { event.stopPropagation(); open('revoke', event.currentTarget); }}>Revoke</button> : null}
  </span>;
}

/** What a Change opens: the revoke form or the give form, for one grant. */
export function ActForm({ w, g, act, done, close }: { w: GrantWorld; g: Grant; act: Act; done: () => void; close: () => void }) {
  return act === 'revoke' ? <Revoke w={w} g={g} done={done} close={close} /> : <Delegate w={w} source={g} done={done} close={close} />;
}

type Open = { grant: string; act: Act; opener: HTMLElement };

/** The panel an act opens in: it takes the keyboard when it opens, Escape closes it, and the button that opened it, named by the press itself, gets the keyboard back. */
export function ActPanel({ label, opener, close, children, className }: { label: string; opener: HTMLElement | null; close: () => void; children: ReactNode; className?: string }) {
  const panel = useRef<HTMLElement>(null);
  useEffect(() => {
    panel.current?.querySelector<HTMLElement>('textarea,input,select,button')?.focus();
    return () => { if (opener?.isConnected) opener.focus(); };
  }, [opener]);
  return <section className={className ? 'act-panel ' + className : 'act-panel'} aria-label={label} ref={panel} onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); close(); } }}>{children}</section>;
}

export function GrantTable({ w, grants, done, give = true, empty = 'No grants.' }: { w: GrantWorld; grants: Grant[]; done: () => void; give?: boolean; empty?: string }) {
  const [open, setOpen] = useState<Open | null>(null);
  const columns = grantColumns(w);
  const close = () => setOpen(null);
  return <table className="usage-table grant-table grant-cols" aria-label="Grants">
    <thead><tr>{columns.map((column) => <th key={column.head}>{column.head}</th>)}<th>Change</th></tr></thead>
    <tbody>
      {grants.map((g) => {
        const mine = open?.grant === g.id ? open.act : null;
        return [<tr key={g.id} data-grant={g.id} style={voidOf(w, g) === null ? undefined : { opacity: 0.72 }}>
          {columns.map((column) => <td key={column.head} data-col={column.head}>{column.cell(g)}</td>)}
          <td data-col="Change"><ChangeButtons w={w} g={g} give={give} open={(act, opener) => setOpen({ grant: g.id, act, opener })} /></td>
        </tr>,
        mine && open ? <tr key={g.id + ':' + mine} className="review-row"><td colSpan={columns.length + 1}>
          <ActPanel label={mine === 'revoke' ? 'Revoke' : 'Give'} opener={open.opener} close={close}><ActForm w={w} g={g} act={mine} done={done} close={close} /></ActPanel>
        </td></tr> : null];
      })}
      {grants.length ? null : <tr><td colSpan={columns.length + 1} className="dim">{empty}</td></tr>}
    </tbody>
  </table>;
}
