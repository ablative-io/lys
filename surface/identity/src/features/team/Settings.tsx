/** One line of the chosen agent's reviewed settings above its terminal, read from its provisioning record; what has no route yet is named as such. */
import { request, useLoad } from '../../api';
import type { ProvisioningAnswer } from '../provisioning/Provisioning';

export function Settings({ agent }: { agent: string }) {
  const load = useLoad(async () => request<ProvisioningAnswer>('/agents/' + encodeURIComponent(agent) + '/provisioning'), 'team-settings:' + agent);
  if (load.status !== 'ok') return <div className="team-settings dim">{load.status === 'refused' ? load.refused.refusal.refusal + ': ' + load.refused.refusal.reason : 'Reading settings'}</div>;
  const { profile } = load.data;
  return <div className="team-settings">
    {profile ? <>
      <span><span className="dim">Profile</span> v{profile.version} {profile.reviewed_by ? <span className="dim">reviewed by {profile.reviewed_by}</span> : <span className="why-not">not reviewed</span>}</span>
      <span><span className="dim">MCP</span> {profile.mcp_servers.length ? profile.mcp_servers.map((server) => <span className="pill" key={server.name}>{server.name}</span>) : 'none'}</span>
      <span><span className="dim">Writes to</span> no route yet (DIRECTORY-074)</span>
    </> : <span className="dim">No provisioning profile recorded.</span>}
    <a href={'#/usage/' + encodeURIComponent(agent)}>Budgets and goals</a>
    <a href={'#/file/' + encodeURIComponent(agent) + '/provisioning'}>All settings</a>
  </div>;
}
