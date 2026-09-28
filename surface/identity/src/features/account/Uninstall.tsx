/**
 * Uninstall Lys, on the administrator's account screen.
 *
 * The service answers GET /uninstall only to the administrator, with the data folder and
 * what removing it loses; anyone else is refused NotAdmitted and sees no control. Uninstall
 * keeps the data folder, so installing Lys again signs the same people in; removing the
 * folder too needs a tick, and then a second tick after the page has named everything that
 * is lost. The service starts Lys's uninstall helper and answers at once; the helper stops
 * Lys, so this page stops answering soon after.
 */
import { useState } from 'react';
import { Refused, request, useLoad } from '../../api';

/** What GET /uninstall answers. */
export interface UninstallPlan {
  data_folder: string;
  lost: string[];
}

const readPlan = () => request<UninstallPlan>('/uninstall');

function Confirm({ plan, cancel }: { plan: UninstallPlan; cancel: () => void }) {
  const [removeData, setRemoveData] = useState(false);
  const [understood, setUnderstood] = useState(false);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<boolean | null>(null);
  const [error, setError] = useState<Refused | null>(null);

  async function uninstall() {
    setBusy(true);
    setError(null);
    try {
      await request<unknown>('/uninstall', { remove_data: removeData, confirmed: removeData && understood });
      setDone(removeData);
    } catch (failure) {
      setError(failure instanceof Refused ? failure : new Refused(0, { refusal: 'Unanswered', reason: String(failure) }));
    } finally {
      setBusy(false);
    }
  }

  if (done !== null) {
    return (
      <p role="status" id="uninstalling">
        Lys is uninstalling{done ? ' and removing its data folder' : ', keeping your data folder'}. This page stops
        answering in a moment; you can close it. Opening Lys again says how it went.
      </p>
    );
  }
  return (
    <div className="uninstall-confirm">
      <label>
        <input type="checkbox" name="remove_data" checked={removeData} disabled={busy}
          onChange={(event) => { setRemoveData(event.target.checked); setUnderstood(false); }} />
        {' '}Also remove my data folder
      </label>
      {removeData ? (
        <div className="why-not" id="uninstall-lost">
          <b>Removing the data folder deletes, from this Mac and for good:</b>
          <ul>{plan.lost.map((lost) => <li key={lost}>{lost}</li>)}</ul>
          <p className="note">The folder is <span className="mono">{plan.data_folder}</span>.</p>
          <label>
            <input type="checkbox" name="understood" checked={understood} disabled={busy}
              onChange={(event) => setUnderstood(event.target.checked)} />
            {' '}I understand this cannot be undone
          </label>
        </div>
      ) : (
        <p className="note">Your data folder is kept, so installing Lys again signs the same people in.</p>
      )}
      {error ? (
        <div role="alert" className="why-not"><b>{error.refusal.refusal}</b><p>{error.refusal.reason}</p></div>
      ) : null}
      <button className="btn danger" type="button" data-act="uninstall" disabled={busy || (removeData && !understood)}
        onClick={() => void uninstall()}>
        {busy ? 'Uninstalling…' : removeData ? 'Uninstall Lys and remove my data' : 'Uninstall Lys'}
      </button>
      {' '}
      <button className="btn" type="button" onClick={cancel} disabled={busy}>Cancel</button>
    </div>
  );
}

export function Uninstall() {
  const load = useLoad(readPlan, 'uninstall');
  const [open, setOpen] = useState(false);
  if (load.status === 'loading') return null;
  if (load.status === 'refused') {
    if (load.refused.refusal.refusal === 'NotAdmitted') return null;
    return (
      <div className="card" id="uninstall">
        <h2>Uninstall Lys</h2>
        <div className="why-not"><b>{load.refused.refusal.refusal}</b><p>{load.refused.refusal.reason}</p></div>
      </div>
    );
  }
  return (
    <div className="card" id="uninstall">
      <h2>Uninstall Lys</h2>
      <div className="note" style={{ margin: '2px 0 6px' }}>
        Stops Lys on this Mac and removes it. Your data folder is kept unless you choose to remove it.
      </div>
      {open ? (
        <Confirm plan={load.data} cancel={() => setOpen(false)} />
      ) : (
        <button className="btn" type="button" data-act="uninstall-open" onClick={() => setOpen(true)}>Uninstall Lys…</button>
      )}
    </div>
  );
}
