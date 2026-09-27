import type { Grant } from '../../generated/grants';
import { cannotGive } from './model';
import type { GrantWorld } from './model';

/** Everything the caller cannot give, each with its reason (conformance 2.4). */
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
        <div className="note"><span className="open-q">not built yet</span> The directory records no service accounts yet, so none is listed here.</div>
      </div>
    </>
  );
}
