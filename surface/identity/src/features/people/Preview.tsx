import { RoleSummary } from '../roles/AssignedRoles';
import type { RolesLoad } from '../roles/AssignedRoles';
import { useShell } from '../../shell/ShellContext';
import type { Entry } from './directory';
import type { Load } from '../../api';
import { Reach } from './reach';
import type { DirectoryReach } from './reach';
import { Pill } from './Pill';

/** The selected row, without leaving the list. */
export function Preview({ x, reach, roles }: { x: Entry; roles: RolesLoad; reach: Load<DirectoryReach> }) {
  const shell = useShell();
  const person = x.person;
  const agent = x.kind === 'agent';
  return (
    <div className="card">
      <div className="row" style={{ padding: '0 0 8px' }}>
        <h2>{x.display_name}</h2>
        <span className={'state ' + x.state}>{x.state}</span>
      </div>
      <div className="sec" style={{ marginBottom: 10 }}>
        {agent ? 'agent' : 'person'}
        {person ? (
          <>
            {' · answers to '}
            <Pill x={person} />
          </>
        ) : null}
      </div>
      <div className="section-h" style={{ marginTop: 6 }}>Roles</div>
      <RoleSummary load={roles} id={x.id} />
      <div className="section-h">Reaches</div>
      <div className="note">
        <Reach load={reach} id={x.id} />
      </div>
      {agent ? (
        <>
          <div className="section-h">Now</div>
          <div className="note">
            <span className="open-q">not built yet</span> sessions show only when a runtime reports them.
          </div>
        </>
      ) : null}
      <div style={{ display: 'flex', gap: 8, marginTop: 14, flexWrap: 'wrap' }}>
        <a className="btn" href={'#/file/' + x.id}>Open file</a>
        <a className="btn" href={'#/graph/' + x.id}>Show in graph</a>
        {agent && x.state === 'active' ? (
          <button className="btn" data-act="start" onClick={() => shell.toast('Starting an agent is not built yet')}>
            Start…
          </button>
        ) : null}
      </div>
    </div>
  );
}
