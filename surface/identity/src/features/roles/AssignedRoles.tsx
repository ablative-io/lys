/** Identity files and directory rows read the assigned version, never substitute the role's latest version. */
import { request, useLoad } from '../../api';
import type { Load } from '../../api';
import { clock } from '../file/time';
import type { Role } from './contract';
import { ErrorWords, PART } from '../people/Words';

export const readRoles = () => request<{ roles: Role[] }>('/roles');
export type RolesLoad = Load<{ roles: Role[] }>;

export function RoleSummary({ load, id }: { load: RolesLoad; id: string }) {
  if (load.status === 'loading') return <span className="dim">Reading roles…</span>;
  if (load.status === 'refused') return <ErrorWords problem={load.refused} />;
  const held = load.data.roles.flatMap((role) => role.holders.filter((holder) => holder.holder === id && holder.state === 'holding').map((holder) => `${role.name} · v${holder.version}`));
  return <span>{held.length ? held.join('; ') : 'No role assigned'}</span>;
}

export function AssignedRoles({ id }: { id: string }) {
  const load = useLoad(readRoles, 'assigned-roles:' + id);
  if (load.status === 'loading') return <p>Reading role assignments…</p>;
  if (load.status === 'refused') return <ErrorWords problem={load.refused} />;
  const holdings = load.data.roles.flatMap((role) => role.holders.filter((holder) => holder.holder === id).map((holder) => ({ role, holder })));
  const current = holdings.filter(({ holder }) => holder.state === 'holding');
  const past = holdings.filter(({ holder }) => holder.state !== 'holding');
  return <>
    {current.length === 0 ? <section className="card"><p>No role is currently assigned to this person or agent.</p><a className="btn" href="#/roles">Open roles</a></section> : null}
    {current.map(({ role, holder }) => {
      const version = role.versions.find((entry) => entry.number === holder.version);
      return <section className="card" key={holder.assignment}><h2><a href={'#/roles/' + encodeURIComponent(role.id)}>{role.name}</a> · Version {holder.version}</h2>
        <p>{holder.ends_at === null ? 'No expiry' : 'Until ' + clock(holder.ends_at)}{holder.behind ? ` · Version ${role.latest} is available; this assignment has not moved.` : ''}</p>
        {version ? <>{(['responsibilities', 'goals', 'practice', 'profile'] as const).map((part) => <section key={part}><h3>{PART[part]}</h3><p style={{ whiteSpace: 'pre-wrap' }}>{version[part] || 'None recorded.'}</p></section>)}<p className="note">Access is determined by grants, separately from this role.</p></>
          : <ErrorWords problem={"RoleVersionMissing: " + role.id + " assignment " + holder.assignment + " names missing version " + holder.version} />}
      </section>;
    })}
    {past.length ? <section className="card" aria-label="Past role assignments"><h3>Past role assignments ({past.length})</h3>{past.map(({ role, holder }) => <p key={holder.assignment}><a href={'#/roles/' + encodeURIComponent(role.id)}>{role.name}</a> · Version {holder.version} · {holder.state}{holder.ended_at === null ? '' : ' · ended ' + clock(holder.ended_at)}</p>)}</section> : null}
  </>;
}
