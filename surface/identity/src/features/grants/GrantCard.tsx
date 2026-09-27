import { useShell } from '../../shell/ShellContext';
import type { Grant } from '../../generated/grants';
import { grantNo, lastsText, nameOf, onText, passText, standing } from './model';
import type { GrantWorld } from './model';

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
export function GrantCard({ w, g, chain }: { w: GrantWorld; g: Grant; chain: Grant[] }) {
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
        <span>Allows: {g.actions.join(', ')}</span>
        <span className="mono">{grantNo(g.id)}</span>
        {!g.revoked ? (
          <a href="#" data-act="revoke" style={{ color: 'var(--danger)' }} onClick={(e) => { e.preventDefault(); shell.toast('Revoking from this screen is not built yet'); }}>
            Revoke
          </a>
        ) : null}
      </div>
      {s.ok ? null : <div className="note" style={{ color: 'var(--danger)', marginTop: 4 }}>{s.why}</div>}
    </div>
  );
}
