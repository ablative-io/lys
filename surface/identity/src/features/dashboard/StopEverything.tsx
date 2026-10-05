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
import type { CordResult, CordView, Listed } from './cord';
import { Act } from '../../shell/Act';

const PULL_KEY = 'lys.pending.stop-everything';
const RELEASE_KEY = 'lys.pending.stop-everything.release';

/** Whether a pull waits for its answer in this tab, so its row opens by itself to ask about it. */
export const pullPending = (): boolean => sessionStorage.getItem(PULL_KEY) !== null;

/**
 * Stop everything, in the head of Running now, for the administrator only. When how the cord stands could not be read
 * the button stays: the off switch is needed most when the page cannot say what runs, and the service refuses a pull
 * from anyone who may not make it.
 */
export function StopEverythingButton({ cord, open }: { cord: CordView | Refused; open: () => void }) {
  if (!(cord instanceof Refused) && !cord.may_pull) return null;
  return <Act symbol="stop" name="Stop everything" word="Stop all" tone="danger" data-act="stop-everything" onClick={open} />;
}

/** The row Stop everything opens: why, whether to kill what does not stop, and then what the pull did. */
export function StopEverythingRow({ columns, close, changed }: { columns: number; close: () => void; changed: () => void }) {
  const [reason, setReason] = useState('');
  const [kill, setKill] = useState(true);
  const [result, setResult] = useState<CordResult | null>(null);
  const change = useRoleChange<CordResult>(PULL_KEY, PULL,
    (answer, body) => answer.operation === body.operation && answer.pulled?.reason === body.reason,
    (answer) => { setResult(answer); changed(); });
  return <tr className="you-asked dash-cord"><td colSpan={columns}>
    {result ? <div role="status" aria-label="What stopping everything did">
      {pullSentences(result).map((sentence, index) => <p key={index}>{sentence}</p>)}
      <Act symbol="close" name="Close" data-act="close" onClick={close} />
    </div> : <form aria-label="Stop everything" onSubmit={(event) => {
      event.preventDefault();
      if (reason.trim()) change.submit({ operation: operationId(), reason: reason.trim(), kill });
    }}>
      <p>This tells every running agent on every computer to stop, and no agent can be started until you let them start again. What stopped and what did not is listed here afterwards.</p>
      <label className="field">Why<input required value={reason} disabled={change.blocked} onChange={(event) => setReason(event.target.value)} placeholder="What happened" /></label>
      <label className="check"><input type="checkbox" checked={kill} disabled={change.blocked} onChange={(event) => setKill(event.target.checked)} /> Kill anything that does not stop. Without this, an agent that ignores the stop keeps running.</label>
      <Act symbol="stop" name="Stop everything now" word="Stop now" tone="danger" type="submit" data-act="stop-everything-now" disabled={change.blocked || !reason.trim()} />{' '}
      <Act symbol="close" name="Cancel" word="Cancel" data-act="close" disabled={change.busy} onClick={close} />
    </form>}
    {result ? null : <ChangeStatus change={change} />}
  </td></tr>;
}

/** The one line across the top while everything is stopped; the administrator may let agents start again from it. */
export function CordLine({ cord, changed, listed }: { cord: CordView | Refused; changed: () => void; listed: Listed }) {
  if (cord instanceof Refused) {
    return <p className="why-not dash-cord-line" role="alert">Lys could not say whether everything is stopped. <small className="refusal-name">{cord.refusal.refusal}</small></p>;
  }
  if (!cord.pulled) return null;
  return <div className="dash-cord-line" role="alert">
    <p>{stoppedLine(cord.pulled, cord.last, listed)}</p>
    {cord.may_pull ? <Release changed={changed} /> : null}
  </div>;
}

function Release({ changed }: { changed: () => void }) {
  const change = useRoleChange<CordView>(RELEASE_KEY, RELEASE,
    (answer, body) => answer.pulled === null && answer.released?.operation === body.operation, changed);
  return <>
    <Act symbol="start" name="Let agents start again" word="Start again" data-act="let-agents-start" disabled={change.blocked} onClick={() => change.submit({ operation: operationId() })} />
    <ChangeStatus change={change} />
  </>;
}
