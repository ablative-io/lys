/**
 * The master off switch on the Dashboard. The administrator presses Stop everything in the head of Running now; the
 * question is a row of that table, and the answer is said in the same row. While everything is stopped one line across
 * the top says who stopped it, when and why, with Let agents start again for the administrator. Each change goes
 * through the keeper: one with no confirmed answer is asked about, never sent twice.
 */
import { useState } from 'react';
import { Refused, operationId } from '../../api';
import { ChangeStatus } from '../roles/ChangeStatus';
import { useRoleChange } from '../roles/useRoleChange';
import { PULL, RELEASE, pullSentences, stoppedLine } from './cord';
import type { CordResult, CordView } from './cord';

const PULL_KEY = 'lys.pending.stop-everything';
const RELEASE_KEY = 'lys.pending.stop-everything.release';

/** Whether a pull waits for its answer in this tab, so its row opens by itself to ask about it. */
export const pullPending = (): boolean => sessionStorage.getItem(PULL_KEY) !== null;

/** Stop everything, in the head of Running now, for the administrator only. */
export function StopEverythingButton({ cord, open }: { cord: CordView | Refused; open: () => void }) {
  if (cord instanceof Refused || !cord.may_pull) return null;
  return <button type="button" className="btn danger" data-act="stop-everything" onClick={open}>Stop everything</button>;
}

/** The row Stop everything opens: why, whether to kill what does not stop, and then what the pull did. */
export function StopEverythingRow({ columns, close, changed }: { columns: number; close: () => void; changed: () => void }) {
  const [reason, setReason] = useState('');
  const [kill, setKill] = useState(false);
  const [result, setResult] = useState<CordResult | null>(null);
  const change = useRoleChange<CordResult>(PULL_KEY, PULL,
    (answer, body) => answer.operation === body.operation && answer.pulled?.reason === body.reason,
    (answer) => { setResult(answer); changed(); });
  return <tr className="you-asked dash-cord"><td colSpan={columns}>
    {result ? <div role="status" aria-label="What stopping everything did">
      {pullSentences(result).map((sentence) => <p key={sentence}>{sentence}</p>)}
      <button type="button" className="btn" data-act="close" onClick={close}>Close</button>
    </div> : <form aria-label="Stop everything" onSubmit={(event) => {
      event.preventDefault();
      if (reason.trim()) change.submit({ operation: operationId(), reason: reason.trim(), kill });
    }}>
      <p>This stops every running agent on every computer. Nothing Lys started is left running.</p>
      <label className="field">Why<input required value={reason} disabled={change.blocked} onChange={(event) => setReason(event.target.value)} placeholder="What happened" /></label>
      <label className="check"><input type="checkbox" checked={kill} disabled={change.blocked} onChange={(event) => setKill(event.target.checked)} /> Kill anything that does not stop</label>
      <button className="btn danger" type="submit" data-act="stop-everything-now" disabled={change.blocked || !reason.trim()}>Stop everything now</button>{' '}
      <button className="btn" type="button" data-act="close" disabled={change.busy} onClick={close}>Cancel</button>
    </form>}
    {result ? null : <ChangeStatus change={change} />}
  </td></tr>;
}

/** The one line across the top while everything is stopped; the administrator may let agents start again from it. */
export function CordLine({ cord, changed }: { cord: CordView | Refused; changed: () => void }) {
  if (cord instanceof Refused) {
    return <p className="why-not dash-cord-line" role="alert">Lys could not say whether everything is stopped. <small className="refusal-name">{cord.refusal.refusal}</small></p>;
  }
  if (!cord.pulled) return null;
  return <div className="dash-cord-line" role="alert">
    <p>{stoppedLine(cord.pulled, cord.last)}</p>
    {cord.may_pull ? <Release changed={changed} /> : null}
  </div>;
}

function Release({ changed }: { changed: () => void }) {
  const change = useRoleChange<CordView>(RELEASE_KEY, RELEASE,
    (answer, body) => answer.pulled === null && answer.released?.operation === body.operation, changed);
  return <>
    <button type="button" className="btn" data-act="let-agents-start" disabled={change.blocked} onClick={() => change.submit({ operation: operationId() })}>Let agents start again</button>
    <ChangeStatus change={change} />
  </>;
}
