import type { Grant } from '../../generated/grants';
import { nameOf, resourceTitle } from './model';
import type { GrantWorld } from './model';

/** The whole path as a sentence, for the cell's title: who holds what, step by step. */
export const chainSentence = (w: GrantWorld, chain: Grant[]): string =>
  chain.map((c) => `${nameOf(w, c.holder)} · ${c.relation} of ${resourceTitle(w, c.resource)}`).join(' → ');

/**
 * The path from a person down to a grant: the holders' names only, joined by
 * arrows. The relation and the resource are the row's own; the full path stays
 * in the title for whoever wants it whole.
 */
export function Chain({ w, chain }: { w: GrantWorld; chain: Grant[] }) {
  return (
    <span className="chain" title={chainSentence(w, chain)}>
      {chain.map((c, i) => (
        <span key={c.id} style={{ display: 'contents' }}>
          {i ? <span className="arr">→</span> : null}
          <span className={`pill ${w.who.get(c.holder)?.kind === 'person' ? 'human' : ''} ${c.standing.stands ? '' : 'off'}`}>{nameOf(w, c.holder)}</span>
        </span>
      ))}
    </span>
  );
}
