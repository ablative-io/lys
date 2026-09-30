/** Whose records a screen shows, kept in the address so a link shows the same thing. */
import { useSearchParams } from 'react-router';
import { readWhose, treeOrder, whoseParam } from './org';
import type { OrgTeam, Whose } from './org';

/** The scope in the address; administrators start at everyone, everyone else at their own. */
export function useWhose(admin: boolean): [Whose, (whose: Whose) => void] {
  const [params, setParams] = useSearchParams();
  const whose = readWhose(params.get('whose'), admin);
  const set = (next: Whose) => {
    const kept = new URLSearchParams(params);
    kept.set('whose', whoseParam(next));
    setParams(kept, { replace: true });
  };
  return [whose, set];
}

/** One control on every list screen: mine, a team and the teams under it, or everyone. */
export function WhoseSelect({ whose, set, teams, admin }: { whose: Whose; set: (whose: Whose) => void; teams: OrgTeam[]; admin: boolean }) {
  const order = treeOrder(teams);
  return <label className="whose sec">Showing{' '}
    <select aria-label="Whose" value={whoseParam(whose)} onChange={(event) => set(readWhose(event.target.value, admin))}>
      <option value="mine">Mine</option>
      {admin ? <option value="all">Everyone</option> : null}
      {order.length ? <optgroup label="Teams">
        {order.map(({ team, depth }) => <option key={team.id} value={'team:' + team.id}>{'  '.repeat(depth) + team.name}</option>)}
      </optgroup> : null}
    </select>
  </label>;
}
