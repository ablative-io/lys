/** The configured integrations served by Lys, with configuration kept apart from health. */
import { ConfigTabs } from '../settings/ConfigTabs';
import { request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { SignInProviders } from './SignInProviders';

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
  const load = useLoad(readConnections, 'connections');
  return <div className="page fill">
    <ConfigTabs on="connections" />
    <div className="head"><div><h1>Services and sign-in providers</h1>
      <p className="sub">The services this Lys installation is configured to use.</p></div>
    </div>
    <Gate load={load} title="Connections" ok={(data) => <div className="pane stack">
      <p className="note">Configured means a service is selected in this installation. It does not confirm that service is reachable now.</p>
      {data.connections.length === 0 ? <p>The service returned no configured integrations.</p> : <div className="card-grid">{data.connections.map((connection) => <section className="card" key={connection.id}>
        <h2>{connection.name}</h2><p>{connection.purpose}</p>
        {connection.endpoint ? <p className="note mono">{connection.endpoint}</p> : null}
        <span className={'pill' + (connection.state === 'unconfigured' ? '' : ' ok')}>{labels[connection.state]}</span>
      </section>)}</div>}
      <SignInProviders />
    </div>} />
  </div>;
}
