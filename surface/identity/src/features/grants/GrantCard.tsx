import { useShell } from '../../shell/ShellContext';
import type { Grant } from '../../generated/grants';
import { grantNo, lastsText, nameOf, onText, passText, standing } from './model';
import type { GrantWorld } from './model';
import { REVOKED, Revoke } from './Revoke';

/** The chain from a person down to a grant, as pills. */
export function Chain({ w, chain }: { w: GrantWorld; chain: Grant[] }) {
  return (
    <span className="chain">
      {chain.map((c, i) => (
        <span key={c.id} style={{ display: 'contents' }}>
          {i ? <span className="arr">→</span> : null}
          <span className={`pill ${w.who.get(c.holder)?.kind === 'person' ? 'human' : ''} ${standing(w, c).ok ? '' : 'off'}`}>
            {nameOf(w, c.holder)} · {c.relation} of {onText(c)}
          </span>
        </span>
      ))}
    </span>
  );
}

/** One grant with its chain, what it lets pass on, how long it lasts, and whether it stands. */
export function GrantCard({ w, g, chain, done }: { w: GrantWorld; g: Grant; chain: Grant[]; done: () => void }) {
  const shell = useShell();
  const s = standing(w, g);
  return (
    <div className="card" style={s.ok ? undefined : { opacity: 0.72 }}>
      <div className="row" style={{ padding: 0 }}>
        <span className="mono"><b style={{ color: 'var(--accent)', fontWeight: 500 }}>{g.relation}</b> of {onText(g)}</span>
        {s.ok ? <span className="dot s-active" title="stands" /> : <span className="verdict-mark no" style={{ fontSize: 9, padding: '1px 6px' }}>void</span>}
      </div>
      <div style={{ marginTop: 8 }}><Chain w={w} chain={chain} /></div>
      <div className="note" style={{ marginTop: 6, display: 'flex', gap: 14, flexWrap: 'wrap' }}>
        <span>Passable: {passText(g.pass_on)}</span>
        <span>Lasts: {lastsText(g)}</span>
        <span className="mono">{grantNo(g.id)}</span>
        {!g.revoked && w.who.get(g.holder)?.state !== 'retired' ? (
          <a href="#" data-act="revoke" data-g={g.id} style={{ color: 'var(--danger)' }} onClick={(e) => { e.preventDefault(); shell.openDrawer(<Revoke w={w} g={g} done={done} />); }}>
            Revoke
          </a>
        ) : null}
      </div>
      {s.ok ? null : (
        <div className="note" style={{ color: 'var(--danger)', marginTop: 4 }}>
          {s.why}
          {s.open ? <> <span className="open-q">what suspension refuses: open</span></> : null}
        </div>
      )}
      {g.revoked ? <Revocation g={g} /> : null}
    </div>
  );
}

/** After a revoke: the policy change, and what is still to design. */
function Revocation({ g }: { g: Grant }) {
  const made = REVOKED.get(g.id);
  return (
    <div className="card" style={{ marginTop: 8, background: 'var(--surface-default)' }}>
      <div className="note">
        {made ? `Policy changed at ${made.at}, change ${made.revision}.` : 'Policy changed.'} Every check from here on refuses.
      </div>
      <div className="row" style={{ padding: '5px 0' }}>
        <span><span className="svc built-in">built in</span></span>
        <span className="note">{made ? `asks with change ${made.revision} or later, so its next check refuses` : 'asks before acting, so its next check refuses'}</span>
      </div>
      <div className="note">Calls already admitted before the change: <span className="open-q">to design</span></div>
    </div>
  );
}
