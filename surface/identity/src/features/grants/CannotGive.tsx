import { request, useLoad } from '../../api';
import type { ServiceAccount } from '../../generated';
import type { Grant } from '../../generated/grants';
import { cannotGive } from './model';
import type { GrantWorld } from './model';

/** Everything the caller cannot give, each with its reason, in the mock-up's order (conformance 2.4). */
export function CannotGive({ w, source }: { w: GrantWorld; source: Grant | null }) {
  return (
    <>
      {cannotGive(w, source).map(([what, why]) => (
        <div className="why-not" key={what}>
          <b>{what}</b>
          <div className="note">{why}</div>
        </div>
      ))}
      <ServiceAccountsHeld />
      <div className="why-not">
        <b>Your sign-in identities</b>
        <div className="note">They prove who you are. No agent can hold them.</div>
      </div>
    </>
  );
}

/** The caller's service-account records, each use-only: a service account is used by its person and never passed to an agent (conformance 1.3, 2.4). */
function ServiceAccountsHeld() {
  const load = useLoad(() => request<{ service_accounts: ServiceAccount[] }>('/service-accounts'), 'cannot-give:service-accounts');
  if (load.status === 'loading') return <div className="why-not"><b>Service accounts</b><div className="note">…</div></div>;
  if (load.status === 'refused') return <div className="why-not"><b>Service accounts</b><div className="note">{load.refused.refusal.refusal}: {load.refused.message}</div></div>;
  const held = load.data.service_accounts.filter((account) => account.state === 'active');
  if (!held.length) return <div className="why-not"><b>Service accounts</b><div className="note">You have no service-account records. A service account is use-only and is never passed to an agent.</div></div>;
  return <>{held.map((account) => <div className="why-not" key={account.id}><b>{account.name}</b><div className="note">Use-only: a service account is used by you and never passed to an agent.</div></div>)}</>;
}
