/**
 * Choosing the folder an agent works in, on the computer it runs on. The
 * folders of that computer are browsed here, on this page, through its
 * runner: the person never needs to be at that computer. A path can also be
 * typed or pasted, with the folders that match offered as they type. Either way the
 * computer itself is asked: a path is taken only once that computer has
 * opened it, so a folder that is missing or unreadable there is said with the
 * computer's own reason and never taken on trust.
 *
 * Nothing is asked until the person presses the button. The computer is the
 * one the agent runs on; only with several computers and none chosen is the
 * person asked which first. Browsing starts in the folder already chosen, or
 * else in the home folder of whoever runs Lys on that computer. When the folder
 * chosen before cannot be read, that is said, with the computer's reason.
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
export async function foldersIn(machine: string, under?: string, signal?: AbortSignal): Promise<FolderList> {
  const answer = await request<unknown>('/network/machines/' + encodeURIComponent(machine) + '/folders', under ? { under } : {}, 'POST', signal);
  if (!readable(answer, machine)) throw new Refused(200, { refusal: 'FoldersUnreadable', reason: 'The computer did not answer with its folders.' });
  return answer;
}

/** The refusals that mean the folder named is not one the computer can open, as opposed to the computer not answering. */
const NOT_A_FOLDER = ['folder_unreadable', 'folder_invalid'];
export const inside = (under: string, name: string) => (under === '/' ? '' : under) + '/' + name;
/** What a person typed, as a path the computer can be asked about: trailing slashes gone, never empty. */
export const typedPath = (typed: string) => typed.trim().replace(/\/+$/, '') || '/';
/** The folder a half-typed path is being typed inside, and the start of the name being typed there. */
export const typingIn = (typed: string): { parent: string; start: string } | null => {
  const text = typed.trim();
  if (!text.startsWith('/')) return null;
  const cut = text.lastIndexOf('/');
  return { parent: text.slice(0, cut) || '/', start: text.slice(cut + 1) };
};

/** The folders above and including `under`, each with the name drawn for it: "/" first, then one per part of the path. */
export const pathParts = (under: string): { path: string; name: string }[] => {
  const names = under.split('/').filter(Boolean);
  return [{ path: '/', name: '/' }, ...names.map((name, index) => ({ path: '/' + names.slice(0, index + 1).join('/'), name }))];
};

export function FolderChooser({ computers, preferred = '', chosen, choose, disabled = false, label, word = 'Choose' }: {
  computers: { id: string; name: string }[]; preferred?: string; chosen: string; choose: (folder: string) => void; disabled?: boolean;
  /** What is said of the button that opens the chooser. */
  label?: string;
  /** The one word drawn beside the symbol that opens the chooser; what is said of it is `label`. */
  word?: string;
}) {
  const known = computers.some((entry) => entry.id === preferred) ? preferred : computers.length === 1 ? computers[0].id : '';
  const [picking, setPicking] = useState('');
  const computer = known || picking;
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Error | null>(null);
  const [gone, setGone] = useState<{ folder: string; problem: Refused } | null>(null);
  const [notice, setNotice] = useState('');
  const [typed, setTyped] = useState(chosen);
  const [typedWrong, setTypedWrong] = useState('');
  const [hints, setHints] = useState<FolderList | null>(null);
  const [here, setHere] = useState<FolderList | null>(null);
  const [reading, setReading] = useState(false);
  const [hidden, setHidden] = useState(false);
  const working = useRef(false);
  const browsing = useRef<AbortController | null>(null);
  const name = computers.find((entry) => entry.id === computer)?.name ?? '';
  useEffect(() => () => browsing.current?.abort(), []);
  // Browsing asks the computer for the folders inside one folder; only the newest ask is shown, so a slow answer never replaces a later one.
  // Opening on the folder chosen before: when that folder is gone or unreadable it is said, and browsing starts at the home folder instead.
  const browse = async (machine: string, under?: string) => {
    browsing.current?.abort();
    const current = new AbortController();
    browsing.current = current;
    setReading(true); setProblem(null); setTypedWrong('');
    try {
      const found = await foldersIn(machine, under, current.signal);
      if (current.signal.aborted) return;
      setHere(found); setTyped(found.under);
    } catch (error) {
      if (current.signal.aborted) return;
      if (under && under === chosen && error instanceof Refused && NOT_A_FOLDER.includes(error.refusal.refusal)) {
        setGone({ folder: chosen, problem: error });
        void browse(machine);
        return;
      }
      if (under && error instanceof Refused && NOT_A_FOLDER.includes(error.refusal.refusal)) setTypedWrong('There is no folder Lys can open at ' + under + ' on ' + name + '.');
      else setProblem(error instanceof Error ? error : new Error(String(error)));
    } finally { if (browsing.current === current) setReading(false); }
  };
  const start = (machine: string) => { setHere(null); setGone(null); void browse(machine, chosen || undefined); };
  // A typed path is taken once the computer has opened it; one that is not there, or cannot be read, is said beside the field in the person's words.
  const go = async (machine: string, path: string) => {
    if (working.current) return;
    working.current = true; setBusy(true); setTypedWrong(''); setNotice('');
    try { const found = await foldersIn(machine, path); setProblem(null); choose(found.under); setOpen(false); }
    catch (error) {
      if (error instanceof Refused && NOT_A_FOLDER.includes(error.refusal.refusal)) setTypedWrong('There is no folder Lys can open at ' + path + ' on ' + name + '. Check the path, or choose it in the list above.');
      else setProblem(error instanceof Error ? error : new Error(String(error)));
    } finally { working.current = false; setBusy(false); }
  };
  // A new parent path starts one read; edits within it filter the returned names.
  const hinting = open && computer ? typingIn(typed) : null;
  const hintParent = hinting ? hinting.parent : null;
  useEffect(() => {
    if (!hintParent || !computer) return;
    const current = new AbortController();
    setHints(null);
    void foldersIn(computer, hintParent, current.signal).then(
      (found) => { if (!current.signal.aborted) setHints(found); },
      (error: unknown) => {
        if (!current.signal.aborted) {
          setHints(null);
          // A half-typed path that names no folder is only typing; the field says so when Open is pressed.
          if (!(error instanceof Refused && NOT_A_FOLDER.includes(error.refusal.refusal))) setProblem(error instanceof Error ? error : new Error(String(error)));
        }
      },
    );
    return () => current.abort();
  }, [hintParent, computer]);
  if (!computers.length) return <p className="folder-chooser">You choose its folder once a computer with Lys running on it is added.</p>;
  if (!open) return <p className="folder-chooser">
    <Act symbol="open" name={label ?? (chosen ? 'Choose another folder' : 'Choose a folder')} word={word} disabled={disabled} onClick={() => { setOpen(true); setTyped(chosen); setNotice(''); setTypedWrong(''); if (computer) start(computer); }} />
  </p>;
  const offered = hinting && hints && hints.machine === computer && hints.under === hinting.parent
    ? hints.folders.filter((entry) => !entry.startsWith('.') && entry.toLowerCase().startsWith(hinting.start.toLowerCase())).slice(0, 50).map((entry) => inside(hints.under, entry)) : [];
  const listId = 'folder-paths-' + (computer || 'none');
  const shown = here && here.machine === computer ? here : null;
  const folders = shown ? shown.folders.filter((entry) => hidden || !entry.startsWith('.')).sort((a, b) => a.localeCompare(b)) : [];
  const hiddenCount = shown ? shown.folders.filter((entry) => entry.startsWith('.')).length : 0;
  return <div className="folder-chooser" role="group" aria-label="Choose a folder">
    {known ? null : <label className="field">Computer the folder is on<select name="folder-computer" value={computer} disabled={busy} onChange={(event) => { setPicking(event.target.value); setHints(null); setNotice(''); setTypedWrong(''); if (event.target.value) start(event.target.value); }}>
      {computer ? null : <option value="">Choose a computer</option>}
      {computers.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
    </select></label>}
    {gone ? <div className="folder-recovery">
      <p>The folder chosen before, <code>{gone.folder}</code>, could not be read on {name}. Choose a readable folder, or ask its owner to check access.</p>
      <ErrorWords problem={gone.problem} />
    </div> : null}
    {computer ? <section className="folder-browser" aria-label={'Folders on ' + name} aria-busy={reading}>
      <h4>Folders on {name}</h4>
      {shown ? <>
        <nav className="folder-path-parts" aria-label="Folders above this one">
          {pathParts(shown.under).map((part) => <Act key={part.path} symbol="back" name={'Go to ' + part.path} word={part.name} disabled={busy || part.path === shown.under} onClick={() => { void browse(computer, part.path); }} />)}
        </nav>
        {folders.length ? <ul className="folder-list">
          {folders.map((entry) => <li key={entry}><Act symbol="open" name={'Open ' + inside(shown.under, entry)} word={entry} disabled={busy} onClick={() => { void browse(computer, inside(shown.under, entry)); }} /></li>)}
        </ul> : <p className="hint">{reading ? 'Reading…' : 'No folders inside ' + shown.under + '.'}</p>}
        <p className="folder-browser-acts">
          <Act symbol="approve" name={'Choose ' + shown.under + ' on ' + name} word="Choose this folder" tone="primary" disabled={busy || reading} onClick={() => { choose(shown.under); setOpen(false); }} />
          {hiddenCount ? <label className="check"><input type="checkbox" checked={hidden} onChange={(event) => setHidden(event.target.checked)} /> Show hidden folders ({hiddenCount})</label> : null}
        </p>
      </> : <p className="hint">{reading ? 'Asking ' + name + ' for its folders…' : 'Lys has not read ' + name + '’s folders yet.'}</p>}
      {!reading && !shown ? <Act symbol="retry" name={'Ask ' + name + ' for its folders again'} word="Try again" disabled={busy} onClick={() => start(computer)} /> : null}
    </section> : null}
    {notice ? <p role="status" className="folder-notice">{notice}</p> : null}
    {computer ? <form className="folder-path" onSubmit={(event) => {
      // This form sits inside the page's own form on Add an agent; its submit must never reach that form, or Open would add the agent.
      event.preventDefault();
      event.stopPropagation();
      if (!typed.trim().startsWith('/')) { setTypedWrong('Type the whole path, starting with /, for example /home/you/Developer/project.'); return; }
      void go(computer, typedPath(typed));
    }}>
      <label className="field">Or type the folder on {name}
        <input name="folder-path" value={typed} list={listId} onChange={(event) => { setTyped(event.target.value); setTypedWrong(''); }} placeholder="/home/you/Developer/project" autoComplete="off" autoCapitalize="off" spellCheck={false} aria-describedby={typedWrong ? listId + '-wrong' : undefined} aria-invalid={typedWrong ? true : undefined} disabled={busy} />
      </label>
      <datalist id={listId}>{offered.map((path) => <option key={path} value={path} />)}</datalist>
      <Act symbol="enter" name="Open the folder typed" word="Open" type="submit" disabled={busy || !typed.trim()} />
      {typedWrong ? <p id={listId + '-wrong'} role="alert" className="why-not">{typedWrong}</p> : null}
    </form> : null}
    {problem ? <div className="folder-problem">
      <p>Lys could not ask {name || 'the computer'} about its folders. Check the computer’s runner, then try again.</p>
      <ErrorWords problem={problem} />
    </div> : null}
    <p><Act symbol="close" name="Cancel" word="Cancel" onClick={() => { browsing.current?.abort(); setOpen(false); }} /></p>
  </div>;
}
