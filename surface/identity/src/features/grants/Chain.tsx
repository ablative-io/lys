import type { Grant } from '../../generated/grants';
import { nameOf, onText } from './model';
import type { GrantWorld } from './model';

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
