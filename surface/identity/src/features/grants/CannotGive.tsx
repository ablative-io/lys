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
      <div className="why-not">
        <b>Service accounts</b>
        <div className="note">A service-account record does not grant permission. Delegate access through a grant; manage account records in <a href="#/service-accounts">Service accounts</a>.</div>
      </div>
      <div className="why-not">
        <b>Your sign-in identities</b>
        <div className="note">They prove who you are. No agent can hold them.</div>
      </div>
    </>
  );
}
