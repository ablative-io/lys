/** The configured integrations served by Lys, with configuration kept apart from health. */
import { useState } from 'react';
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';

interface Connection {
  id: string;
  name: string;
  purpose: string;
  state: 'configured' | 'local' | 'unconfigured';
  endpoint: string | null;
}
interface ConnectionsView { connections: Connection[]; health_checked: false }
const readConnections = () => request<ConnectionsView>('/connections');
const labels = { configured: 'Configured', local: 'Local to Lys', unconfigured: 'Not configured' };

export function Connections() {
  const [revision, setRevision] = useState(0);
  const load = useLoad(readConnections, 'connections-' + revision);
  return <div className="page">
    <div className="head"><div><div className="eyebrow">Runtime</div><h1>Connections</h1>
      <p className="sub">The services this Lys installation is configured to use.</p></div>
      <button className="btn" onClick={() => setRevision((value) => value + 1)}>Refresh</button>
    </div>
    <Gate load={load} title="Connections" ok={(data) => <>
      <p>Configured means a service is selected in this installation. It does not confirm that service is reachable now.</p>
      <div className="grid2">{data.connections.map((connection) => <section className="card" key={connection.id}>
        <h2>{connection.name}</h2><p>{connection.purpose}</p>
        <p><strong>{labels[connection.state]}</strong></p>
        {connection.endpoint ? <details><summary>Connection details</summary><p className="mono">{connection.endpoint}</p></details> : null}
      </section>)}</div>
      {data.connections.length === 0 ? <p>The service returned no configured integrations.</p> : null}
      <p className="note">This view shows installation settings. It does not yet list products using Lys or external accounts available to agents.</p>
      <a className="btn" href="#/service-accounts">Manage service accounts</a>{' '}
      <a className="btn" href="#/me">Your sign-in accounts</a>
    </>} />
  </div>;
}
