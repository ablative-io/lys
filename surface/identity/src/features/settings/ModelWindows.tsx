/**
 * The context window a person declares for each model. A call's context is shown as a share of the window declared
 * for its model, so a model with no row here sets no context figure; one ticked as side work never does. Models Lys
 * has seen calls for and has no row for are listed to be filled in. The whole table is saved at once.
 */
import { useState } from 'react';
import { Refused, request } from '../../api';

/** The organisation setting as the service keeps it. */
export interface Organisation { zone: string; version: number; model_windows?: Record<string, number | null> }

interface Row { model: string; window: string; side: boolean }

/** A count of tokens as a person writes one: digits, with commas, spaces or underscores between them if they like. */
const tokens = (written: string): number | null => {
  const digits = written.replace(/[,_\s]/g, '');
  return /^[1-9]\d*$/.test(digits) ? Number(digits) : null;
};
const rowsOf = (organisation: Organisation, undeclared: string[]): Row[] => [
  ...Object.entries(organisation.model_windows ?? {}).map(([model, window]) => ({ model, window: window === null ? '' : window.toLocaleString('en-AU'), side: window === null })),
  ...undeclared.map((model) => ({ model, window: '', side: false })),
];

export function ModelWindows({ organisation, undeclared, saved }: { organisation: Organisation; undeclared: string[]; saved: () => void }) {
  const [rows, setRows] = useState<Row[]>(() => rowsOf(organisation, undeclared));
  const [fresh, setFresh] = useState('');
  const [refused, setRefused] = useState<Refused | null>(null);
  const [busy, setBusy] = useState(false);
  const set = (model: string, change: Partial<Row>) => setRows((now) => now.map((row) => row.model === model ? { ...row, ...change } : row));
  // A row with a window written that is not a count of tokens is said beside it, and nothing is sent until it is one.
  const wrong = rows.filter((row) => !row.side && row.window.trim() !== '' && tokens(row.window) === null).map((row) => row.model);
  const name = fresh.trim();
  const add = () => { if (name && !rows.some((row) => row.model === name)) { setRows((now) => [...now, { model: name, window: '', side: false }]); setFresh(''); } };
  const save = () => {
    // A row with neither a window nor the tick declares nothing, and is left out.
    const table = Object.fromEntries(rows.filter((row) => row.side || row.window.trim() !== '').map((row) => [row.model, row.side ? null : tokens(row.window)]));
    setBusy(true); setRefused(null);
    request('/configuration', { zone: organisation.zone, version: organisation.version, model_windows: table }, 'PUT')
      .then(saved, (problem: unknown) => setRefused(problem instanceof Refused ? problem : new Refused(0, { refusal: 'Unanswered', reason: String(problem) })))
      .finally(() => setBusy(false));
  };
  return <>
    <h2>Model context windows</h2>
    <p className="note">An agent's context is shown as a share of the window declared here for the model each call used. A model with no window sets no context figure. Tick side work for a small model a harness uses for its own errands, so its calls never replace an agent's context.</p>
    <table className="usage-list usage-table model-windows"><thead><tr><th>Model</th><th>Context window, in tokens</th><th>Side work</th><th></th></tr></thead><tbody>
      {rows.map((row) => <tr key={row.model} data-model={row.model}>
        <td>{row.model}{undeclared.includes(row.model) ? <small className="dim"> Seen in calls; nothing declared.</small> : null}</td>
        <td><input type="text" inputMode="numeric" aria-label={'Context window of ' + row.model} value={row.side ? '' : row.window} disabled={row.side} onChange={(event) => set(row.model, { window: event.target.value })} />
          {wrong.includes(row.model) ? <small className="why-not" role="alert"> Write a count of tokens, such as 200,000.</small> : null}</td>
        <td><input type="checkbox" aria-label={row.model + ' is side work'} checked={row.side} onChange={(event) => set(row.model, { side: event.target.checked })} /></td>
        <td><button type="button" className="btn" data-act="remove-model" onClick={() => setRows((now) => now.filter((each) => each.model !== row.model))}>Remove</button></td></tr>)}
      <tr className="add"><td><input type="text" aria-label="Another model's name" placeholder="Model name, as its calls name it" value={fresh} onChange={(event) => setFresh(event.target.value)}
        onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); add(); } }} /></td>
        <td colSpan={2}></td><td><button type="button" className="btn" data-act="add-model" disabled={!name || rows.some((row) => row.model === name)} onClick={add}>Add</button></td></tr>
    </tbody></table>
    <p><button type="button" className="btn primary" data-act="save-model-windows" disabled={busy || wrong.length > 0} onClick={save}>{busy ? 'Saving…' : 'Save the table'}</button>
      {refused ? <span className="why-not" role="alert"> The table was not saved. <small className="refusal-name" title={refused.refusal.reason}>{refused.refusal.refusal}</small> {refused.refusal.reason}</span> : null}</p>
  </>;
}
