import { useShell } from '../../shell/ShellContext';
import type { Grant } from '../../generated/grants';
import { clock, day } from '../file/time';
import { grantNo, lastUsedText, nameOf, onText, passText, voidOf } from './model';
import type { GrantWorld } from './model';
import { Revoke } from './Revoke';

/** The chain from a person down to a grant, as pills. */
export function Chain({ w, chain }: { w: GrantWorld; chain: Grant[] }) {
  return (
    <span className="chain">
      {chain.map((c, i) => (
        <span key={c.id} style={{ display: 'contents' }}>
          {i ? <span className="arr">→</span> : null}
          <span className={`pill ${w.who.get(c.holder)?.kind === 'person' ? 'human' : ''} ${c.standing.stands ? '' : 'off'}`}>
            {nameOf(w, c.holder)} · {c.relation} of {onText(c)}
          </span>
        </span>
      ))}
    </span>
  );
}

/** One grant with its chain, what it lets pass on, its window, and whether it stands, as the service answered them. */
export function GrantCard({ w, g, chain, done }: { w: GrantWorld; g: Grant; chain: Grant[]; done: () => void }) {
  const shell = useShell();
  const v = voidOf(w, g);
  return (
    <div className="card" style={v === null ? undefined : { opacity: 0.72 }}>
      <div className="row" style={{ padding: 0 }}>
        <span className="mono"><b style={{ color: 'var(--accent)', fontWeight: 500 }}>{g.relation}</b> of {onText(g)}</span>
        {v === null ? <span className="dot s-active" title="stands" /> : <span className="verdict-mark no" style={{ fontSize: 9, padding: '1px 6px' }}>void</span>}
      </div>
      <div style={{ marginTop: 8 }}><Chain w={w} chain={chain} /></div>
      <div className="note" style={{ marginTop: 6, display: 'flex', gap: 14, flexWrap: 'wrap' }}>
        <span>Passable: {passText(g.pass_on)}</span>
        <span>Window: {g.effective_ends_at === null ? `from ${day(g.window.starts_at)}, no end` : `${day(g.window.starts_at)} to ${day(g.effective_ends_at)}`}</span>
        <span>Last used: {lastUsedText(g)}</span>
        <span className="mono">{grantNo(g.id)}</span>
        {!g.revoked && w.who.get(g.holder)?.state !== 'retired' ? (
          <a href="#" data-act="revoke" data-g={g.id} style={{ color: 'var(--danger)' }} onClick={(e) => { e.preventDefault(); shell.openDrawer(<Revoke w={w} g={g} done={done} />); }}>
            Revoke
          </a>
        ) : null}
      </div>
      {v === null ? null : (
        <div className="note" style={{ color: 'var(--danger)', marginTop: 4 }}>
          {v.why}
        </div>
      )}
      {g.revoked ? <Revocation g={g} /> : null}
    </div>
  );
}

/** After a revoke: the policy change. */
function Revocation({ g }: { g: Grant }) {
  const made = g.revoked_at !== null && g.revoked_revision !== null ? { at: clock(g.revoked_at).split(' ').at(-1) ?? '', revision: g.revoked_revision } : null;
  return (
    <div className="card" style={{ marginTop: 8, background: 'var(--surface-default)' }}>
      <div className="note">
        {made ? `Policy changed at ${made.at}, change ${made.revision}.` : 'Policy changed.'} Every check from here on refuses.
      </div>
      <div className="row" style={{ padding: '5px 0' }}>
        <span><span className="svc built-in">built in</span></span>
        <span className="note">{made ? `asks with change ${made.revision} or later, so its next check refuses` : 'asks before acting, so its next check refuses'}</span>
      </div>
    </div>
  );
}
