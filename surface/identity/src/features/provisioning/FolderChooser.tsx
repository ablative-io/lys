/**
 * Choosing a folder on a computer Lys runs agents on. A person types or pastes
 * its path, with the folders that match offered as they type, or opens folders
 * one at a time and climbs back up the path by any of its parts. Either way the
 * computer itself is asked: a folder is taken only once that computer has
 * listed it, so a path that is missing or unreadable there is said with the
 * computer's own reason and never taken on trust.
 *
 * Nothing is asked until the person presses the button. The first look is in
 * the folder already chosen, or else in the home folder of whoever runs Lys on
 * that computer. When the folder chosen before cannot be read, that is said,
 * with the computer's reason, and the home folder is shown. Folders whose names
 * begin with a dot are the computer's own and are not offered. With several
 * computers and none preferred, the person says which computer first.
 */
import { useEffect, useRef, useState } from 'react';
import { Refused, request } from '../../api';
import { Act } from '../../shell/Act';
import { ErrorWords } from '../people/Words';

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
const nameOf = (folder: string) => folder.slice(folder.lastIndexOf('/') + 1) || folder;
/** Every folder from the top down to `under`, each with its name: the parts of the path a person can go back to. */
export const trail = (under: string): { path: string; name: string }[] =>
  under.split('/').filter(Boolean).map((part, index, parts) => ({ path: '/' + parts.slice(0, index + 1).join('/'), name: part }));
/** What a person typed, as a path the computer can be asked about: trailing slashes gone, never empty. */
export const typedPath = (typed: string) => typed.trim().replace(/\/+$/, '') || '/';
/** The folder a half-typed path is being typed inside, and the start of the name being typed there. */
export const typingIn = (typed: string): { parent: string; start: string } | null => {
  const text = typed.trim();
  if (!text.startsWith('/')) return null;
  const cut = text.lastIndexOf('/');
  return { parent: text.slice(0, cut) || '/', start: text.slice(cut + 1) };
};

export function FolderChooser({ computers, preferred = '', chosen, choose, disabled = false, label, word = 'Choose', confirm = 'Work in' }: {
  computers: { id: string; name: string }[]; preferred?: string; chosen: string; choose: (folder: string) => void; disabled?: boolean;
  /** The words on the button that opens the chooser, and the verb on the one that takes the folder, when the folder is not the one the agent works in. */
  label?: string; confirm?: string;
  /** The one word drawn beside the symbol that opens the chooser; what is said of it is `label`. */
  word?: string;
}) {
  const known = computers.some((entry) => entry.id === preferred) ? preferred : computers.length === 1 ? computers[0].id : '';
  const [picked, setPicked] = useState('');
  const computer = picked || known;
  const [list, setList] = useState<FolderList | null>(null);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Error | null>(null);
  const [gone, setGone] = useState<{ folder: string; problem: Refused } | null>(null);
  const [typed, setTyped] = useState('');
  const [typedWrong, setTypedWrong] = useState('');
  const [hints, setHints] = useState<FolderList | null>(null);
  const working = useRef(false);
  const look = async (machine: string, under?: string, first = false) => {
    if (working.current) return;
    working.current = true; setBusy(true); setProblem(null); setTypedWrong('');
    try {
      let next: FolderList;
      if (first) setGone(null);
      try { next = await foldersIn(machine, under); }
      catch (error) {
        // Only a folder that is gone or unreadable falls back to the home folder, and it is said; any other failure is shown as itself.
        if (!first || !under || !(error instanceof Refused) || !['folder_unreadable', 'folder_invalid'].includes(error.refusal.refusal)) throw error;
        setGone({ folder: under, problem: error });
        next = await foldersIn(machine);
      }
      setList(next); setTyped(next.under);
    } catch (error) {
      setProblem(error instanceof Error ? error : new Error(String(error)));
    } finally { working.current = false; setBusy(false); }
  };
  // A typed path is answered in the person's words: a folder that is not there, or cannot be read, is said beside the field and the folder shown stays as it was.
  const go = async (machine: string, path: string) => {
    if (working.current) return;
    working.current = true; setBusy(true); setTypedWrong('');
    try { const next = await foldersIn(machine, path); setList(next); setTyped(next.under); setProblem(null); }
    catch (error) {
      if (error instanceof Refused && ['folder_unreadable', 'folder_invalid'].includes(error.refusal.refusal)) setTypedWrong('There is no folder Lys can open at ' + path + ' on this computer. Check the path, or choose from the folders below.');
      else setProblem(error instanceof Error ? error : new Error(String(error)));
    } finally { working.current = false; setBusy(false); }
  };
  // While a path is typed, the folders inside the folder it is being typed in are read once a pause in typing comes, to offer the names that match.
  const hinting = open && computer && list ? typingIn(typed) : null;
  const hintParent = hinting && hinting.parent !== list?.under ? hinting.parent : null;
  useEffect(() => {
    if (!hintParent || !computer) return;
    let live = true;
    const timer = setTimeout(() => { foldersIn(computer, hintParent).then((found) => { if (live) setHints(found); }, () => { if (live) setHints(null); }); }, 250);
    return () => { live = false; clearTimeout(timer); };
  }, [hintParent, computer]);
  if (!computers.length) return <p className="folder-chooser">You choose its folder once a computer with Lys running on it is added.</p>;
  const name = computers.find((entry) => entry.id === computer)?.name ?? '';
  if (!open) return <p className="folder-chooser">
    <Act symbol="open" name={label ?? (chosen ? 'Choose another folder' : 'Choose a folder')} word={word} disabled={disabled} onClick={() => { setOpen(true); if (computer) void look(computer, chosen || undefined, true); }} />
  </p>;
  const shown = list ? list.folders.filter((entry) => !entry.startsWith('.')) : [];
  const source = hinting ? (hinting.parent === list?.under ? list : hints?.under === hinting.parent ? hints : null) : null;
  const offered = hinting && source ? source.folders.filter((entry) => !entry.startsWith('.') && entry.toLowerCase().startsWith(hinting.start.toLowerCase())).slice(0, 50).map((entry) => inside(source.under, entry)) : [];
  const listId = 'folder-paths-' + (computer || 'none');
  return <div className="folder-chooser" role="group" aria-label="Choose a folder">
    <label className="field">Computer the folder is on<select name="folder-computer" value={computer} disabled={busy} onChange={(event) => { setPicked(event.target.value); setList(null); setTyped(''); if (event.target.value) void look(event.target.value, undefined, true); }}>
      {computer ? null : <option value="">Choose a computer</option>}
      {computers.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
    </select></label>
    {computer && list ? <form className="folder-path" onSubmit={(event) => {
      event.preventDefault();
      if (!typed.trim().startsWith('/')) { setTypedWrong('Type the whole path, starting with /, for example ' + (list.under || '/Users/you') + '/project.'); return; }
      void go(computer, typedPath(typed));
    }}>
      <label className="field">Folder on {name}
        <input name="folder-path" value={typed} list={listId} onChange={(event) => { setTyped(event.target.value); setTypedWrong(''); }} placeholder="/Users/you/Developer/project" autoComplete="off" autoCapitalize="off" spellCheck={false} disabled={busy} aria-describedby={typedWrong ? listId + '-wrong' : undefined} />
      </label>
      <datalist id={listId}>{offered.map((path) => <option key={path} value={path} />)}</datalist>
      <Act symbol="enter" name="Open the folder typed" word="Open" type="submit" disabled={busy || !typed.trim() || typedPath(typed) === list.under} />
      {typedWrong ? <p id={listId + '-wrong'} role="alert" className="why-not">{typedWrong}</p> : null}
    </form> : null}
    {busy && !list ? <p role="status">Looking at the folders on {name}…</p> : null}
    {gone ? <div className="folder-recovery">
      <p>The folder chosen before, <code>{gone.folder}</code>, could not be read. Lys asked for the computer’s home folder instead. Choose a readable folder or ask its owner to check access.</p>
      <ErrorWords problem={gone.problem} />
    </div> : null}
    {list ? <>
      <p className="folder-where">On {name}, in <code>{list.under}</code></p>
      <nav className="folder-trail" aria-label="The path to this folder">
        {[{ path: '/', name: '/' }, ...trail(list.under)].slice(0, -1).map((part) => <Act key={part.path} symbol="back" name={'Back to ' + (part.path === '/' ? 'the top of the computer' : part.name)} word={part.name} disabled={busy} onClick={() => { void look(list.machine, part.path); }} />)}
        {list.under === '/' ? <span aria-current="location">/</span> : <span aria-current="location">{nameOf(list.under)}</span>}
      </nav>
      {shown.length ? <ul className="folders" aria-label={'Folders inside ' + nameOf(list.under)}>{shown.map((entry) => <li key={entry}>
        <button type="button" className="btn" data-choice="" disabled={busy} onClick={() => { void look(list.machine, inside(list.under, entry)); }}>{entry}</button>
      </li>)}</ul> : <p>There are no folders inside this one.</p>}
      <p><Act symbol="approve" name={confirm + ' ' + nameOf(list.under)} word={confirm + ' ' + nameOf(list.under)} tone="primary" disabled={busy} onClick={() => { choose(list.under); setOpen(false); }} /></p>
    </> : null}
    {problem ? <div className="folder-problem">
      <p>Lys could not look at the folders there. Check the computer’s runner, then close and reopen this chooser to try again.</p>
      <ErrorWords problem={problem} />
    </div> : null}
    <p><Act symbol="close" name="Cancel" word="Cancel" disabled={busy} onClick={() => setOpen(false)} /></p>
  </div>;
}
