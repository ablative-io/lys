/** Review targets the displayed profile version; a pre-existing review is never attributed to the current request. */
import { useRef, useState } from 'react';
import { operationId } from '../../api';
import { useRoleChange } from '../roles/useRoleChange';
import { ChangeStatus } from '../roles/ChangeStatus';
import type { ProvisioningAnswer, ProvisioningProfile } from './Provisioning';
export function ReviewProfile({ agent, person, profile, changed }: { agent: string; person: string; profile: ProvisioningProfile; changed: (message: string) => void }) {
  const result = useRef('');
  const key = 'lys.pending.profile-review.' + person + '.' + agent;
  const prefix = '/agents/' + encodeURIComponent(agent) + '/provisioning/';
  const [version] = useState(() => {
    const raw = sessionStorage.getItem(key);
    if (!raw) return profile.version;
    try {
      const held = JSON.parse(raw) as { path?: unknown };
      if (typeof held.path === 'string' && held.path.startsWith(prefix)) {
        const suffix = held.path.slice(prefix.length);
        if (/^[1-9][0-9]*\/review$/.test(suffix)) return Number(suffix.split('/')[0]);
      }
    } catch {
      // useRoleChange reads these same bytes and names the unreadable record; it blocks admission.
    }
    return profile.version;
  });
  const path = prefix + version + '/review';
  const change = useRoleChange<ProvisioningAnswer>(key, path, (answer, body) => {
    if (answer.agent !== agent || answer.recorded?.version !== version || typeof answer.recorded.operation !== 'string') return false;
    result.current = answer.recorded.operation === body.operation ? 'Version ' + version + ' of these settings is approved.' : 'Version ' + version + ' was already approved by someone else. This did not replace that approval.';
    return true;
  }, () => changed(result.current));
  return <section className="card"><p>A start is refused until the latest settings are approved. Approving does not start the agent; it lets the next start use version {version} of these settings.</p>
    {profile.version === version && profile.reviewed_by ? <p>Approved by {profile.reviewed_by}.{profile.self_reviewed ? ' They also wrote this version.' : ''}</p> : <button className="btn primary" disabled={change.blocked} onClick={() => change.submit({ operation: operationId() })}>Approve these settings</button>}
    <ChangeStatus change={change} />
  </section>;
}
