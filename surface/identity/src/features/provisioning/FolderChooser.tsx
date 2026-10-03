/**
 * Choosing the folder an agent works in. A person never types a path: the
 * computer the agent runs on names its own folders, one folder at a time,
 * and the person opens them until they are in the one they want.
 *
 * Nothing is asked until the person presses the button. The first look is
 * in the folder already chosen, or else in the home folder of whoever runs
 * Lys on that computer; nothing else is assumed about any computer. When
 * the folder chosen before cannot be read, that is said, with the
 * computer's reason, and the home folder is shown.
 * Folders whose names begin with a dot are the computer's own and are not
 * offered. With several computers and none preferred, the person says which
 * computer first, from the ones Lys knows.
 */
import { useRef, useState } from 'react';
import { Refused, request } from '../../api';

/** The folders inside one folder of a computer, as its runner answers. */
export interface FolderList { machine: string; under: string; folders: string[] }

const readable = (value: unknown, machine: string): value is FolderList => typeof value === 'object' && value !== null
  && (value as FolderList).machine === machine && typeof (value as FolderList).under === 'string' && (value as FolderList).under.startsWith('/')
  && Array.isArray((value as FolderList).folders) && (value as FolderList).folders.every((name) => typeof name === 'string');

/** Ask `machine` for the folders inside `under`, or inside its home folder when none is named. */
export async function foldersIn(machine: string, under?: string): Promise<FolderList> {
  const answer = await request<unknown>('/network/machines/' + encodeURIComponent(machine) + '/folders', under ? { under } : {});
  if (!readable(answer, machine)) throw new Refused(200, { refusal: 'FoldersUnreadable', reason: 'The computer did not answer with its folders.' });
  return answer;
}

const inside = (under: string, name: string) => (under === '/' ? '' : under) + '/' + name;
/** The folder above `under`, or none when that is the top of the computer, which is never listed. */
const above = (under: string) => { const parent = under.slice(0, under.lastIndexOf('/')); return parent || null; };
const nameOf = (folder: string) => folder.slice(folder.lastIndexOf('/') + 1) || folder;

export function FolderChooser({ computers, preferred = '', chosen, choose, disabled = false }: {
  computers: { id: string; name: string }[]; preferred?: string; chosen: string; choose: (folder: string) => void; disabled?: boolean;
}) {
  const known = computers.some((entry) => entry.id === preferred) ? preferred : computers.length === 1 ? computers[0].id : '';
  const [picked, setPicked] = useState('');
  const computer = known || picked;
  const [list, setList] = useState<FolderList | null>(null);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<{ refusal: string; reason: string } | null>(null);
  const [gone, setGone] = useState<{ folder: string; refusal: string; reason: string } | null>(null);
  const working = useRef(false);
  const look = async (machine: string, under?: string, first = false) => {
    if (working.current) return;
    working.current = true; setBusy(true); setProblem(null);
    try {
      let next: FolderList;
      if (first) setGone(null);
      try { next = await foldersIn(machine, under); }
      catch (error) {
        // Only a folder that is gone or unreadable falls back to the home folder, and it is said; any other failure is shown as itself.
        if (!first || !under || !(error instanceof Refused) || !['folder_unreadable', 'folder_invalid'].includes(error.refusal.refusal)) throw error;
        setGone({ folder: under, refusal: error.refusal.refusal, reason: error.refusal.reason });
        next = await foldersIn(machine);
      }
      setList(next);
    } catch (error) {
      setProblem(error instanceof Refused ? { refusal: error.refusal.refusal, reason: error.refusal.reason } : { refusal: 'Unexpected', reason: String(error) });
    } finally { working.current = false; setBusy(false); }
  };
  if (!computers.length) return <p className="folder-chooser">You choose its folder once a computer with Lys running on it is added.</p>;
  const name = computers.find((entry) => entry.id === computer)?.name ?? '';
  if (!open) return <p className="folder-chooser">
    <button type="button" className="btn" disabled={disabled} onClick={() => { setOpen(true); if (computer) void look(computer, chosen || undefined, true); }}>{chosen ? 'Choose another folder' : 'Choose a folder'}</button>
  </p>;
  const parent = list ? above(list.under) : null;
  const shown = list ? list.folders.filter((entry) => !entry.startsWith('.')) : [];
  return <div className="folder-chooser" role="group" aria-label="Choose a folder">
    {!known ? <label className="field">Computer the folder is on<select name="folder-computer" value={picked} disabled={busy} onChange={(event) => { setPicked(event.target.value); setList(null); if (event.target.value) void look(event.target.value, undefined, true); }}>
      <option value="">Choose a computer</option>
      {computers.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
    </select></label> : null}
    {busy && !list ? <p role="status">Looking at the folders on {name}…</p> : null}
    {gone ? <p role="alert">The folder chosen before, <code>{gone.folder}</code>, could not be read. {gone.reason} <small className="refusal-name">{gone.refusal}</small></p> : null}
    {list ? <>
      <p>On {name}, in <code>{list.under}</code></p>
      {parent ? <p><button type="button" className="btn" disabled={busy} onClick={() => { void look(list.machine, parent); }}>Back to {nameOf(parent)}</button></p> : null}
      {shown.length ? <ul className="folders">{shown.map((entry) => <li key={entry}>
        <button type="button" className="btn" disabled={busy} onClick={() => { void look(list.machine, inside(list.under, entry)); }}>{entry}</button>
      </li>)}</ul> : <p>There are no folders inside this one.</p>}
      <p><button type="button" className="btn primary" disabled={busy} onClick={() => { choose(list.under); setOpen(false); }}>Work in {nameOf(list.under)}</button></p>
    </> : null}
    {problem ? <p role="alert">Lys could not look at the folders there. {problem.reason} <small className="refusal-name">{problem.refusal}</small></p> : null}
    <p><button type="button" className="btn" disabled={busy} onClick={() => setOpen(false)}>Cancel</button></p>
  </div>;
}
