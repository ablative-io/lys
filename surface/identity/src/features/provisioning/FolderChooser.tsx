/**
 * Choosing the folder an agent works in, on the computer it runs on. The
 * computer's own folder dialog opens on that computer's screen, through its
 * runner, and the folder picked there is taken; or a person types or pastes
 * a path, with the folders that match offered as they type. Either way the
 * computer itself is asked: a path is taken only once that computer has
 * opened it, so a folder that is missing or unreadable there is said with the
 * computer's own reason and never taken on trust.
 *
 * Nothing is asked until the person presses the button. The computer is the
 * one the agent runs on; only with several computers and none chosen is the
 * person asked which first. The dialog opens in the folder already chosen, or
 * else in the home folder of whoever runs Lys on that computer. When the folder
 * chosen before cannot be read, that is said, with the computer's reason.
 */
import { useEffect, useRef, useState } from 'react';
import { Refused, request } from '../../api';
import { Act } from '../../shell/Act';
import { ErrorWords } from '../people/Words';

/** The folders inside one folder of a computer, as its runner answers. */
export interface FolderList { machine: string; under: string; folders: string[] }

/** The folder picked in a computer's own dialog, as its runner answers. */
export interface FolderChosen { machine: string; folder: string }

const readable = (value: unknown, machine: string): value is FolderList => typeof value === 'object' && value !== null
  && (value as FolderList).machine === machine && typeof (value as FolderList).under === 'string' && (value as FolderList).under.startsWith('/')
  && Array.isArray((value as FolderList).folders) && (value as FolderList).folders.every((name) => typeof name === 'string');

const picked = (value: unknown, machine: string): value is FolderChosen => typeof value === 'object' && value !== null
  && (value as FolderChosen).machine === machine && typeof (value as FolderChosen).folder === 'string' && (value as FolderChosen).folder.startsWith('/');

/** Ask `machine` for the folders inside `under`, or inside its home folder when none is named. */
export async function foldersIn(machine: string, under?: string): Promise<FolderList> {
  const answer = await request<unknown>('/network/machines/' + encodeURIComponent(machine) + '/folders', under ? { under } : {});
  if (!readable(answer, machine)) throw new Refused(200, { refusal: 'FoldersUnreadable', reason: 'The computer did not answer with its folders.' });
  return answer;
}

/** Open `machine`'s own folder dialog, starting in `start` or its home folder, and wait for the folder picked there. */
export async function folderChosenOn(machine: string, start?: string): Promise<FolderChosen> {
  const answer = await request<unknown>('/network/machines/' + encodeURIComponent(machine) + '/choose-folder', start ? { start } : {});
  if (!picked(answer, machine)) throw new Refused(200, { refusal: 'FolderChoiceUnreadable', reason: 'The computer did not answer with the folder picked.' });
  return answer;
}

/** The refusals that mean the folder named is not one the computer can open, as opposed to the computer not answering. */
const NOT_A_FOLDER = ['folder_unreadable', 'folder_invalid'];
const inside = (under: string, name: string) => (under === '/' ? '' : under) + '/' + name;
/** What a person typed, as a path the computer can be asked about: trailing slashes gone, never empty. */
export const typedPath = (typed: string) => typed.trim().replace(/\/+$/, '') || '/';
/** The folder a half-typed path is being typed inside, and the start of the name being typed there. */
export const typingIn = (typed: string): { parent: string; start: string } | null => {
  const text = typed.trim();
  if (!text.startsWith('/')) return null;
  const cut = text.lastIndexOf('/');
  return { parent: text.slice(0, cut) || '/', start: text.slice(cut + 1) };
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
  const [dialogOpen, setDialogOpen] = useState(false);
  const [problem, setProblem] = useState<Error | null>(null);
  const [gone, setGone] = useState<{ folder: string; problem: Refused } | null>(null);
  const [notice, setNotice] = useState('');
  const [typed, setTyped] = useState(chosen);
  const [typedWrong, setTypedWrong] = useState('');
  const [hints, setHints] = useState<FolderList | null>(null);
  const working = useRef(false);
  const name = computers.find((entry) => entry.id === computer)?.name ?? '';
  // On opening, the folder chosen before is looked at once, so a folder that is gone or unreadable is said before the dialog starts there.
  const look = async (machine: string) => {
    if (working.current || !chosen) return;
    working.current = true; setBusy(true); setProblem(null); setGone(null);
    try { await foldersIn(machine, chosen); }
    catch (error) {
      if (error instanceof Refused && NOT_A_FOLDER.includes(error.refusal.refusal)) setGone({ folder: chosen, problem: error });
      else setProblem(error instanceof Error ? error : new Error(String(error)));
    } finally { working.current = false; setBusy(false); }
  };
  // The computer's own dialog: this page waits for the person to pick there. A cancel there is said and changes nothing.
  const dialog = async (machine: string) => {
    if (working.current) return;
    working.current = true; setBusy(true); setDialogOpen(true); setProblem(null); setNotice(''); setTypedWrong('');
    try {
      const answer = await folderChosenOn(machine, gone ? undefined : chosen || undefined);
      choose(answer.folder); setOpen(false);
    } catch (error) {
      if (error instanceof Refused && error.refusal.refusal === 'folder_choice_cancelled') setNotice('Cancelled on ' + name + '. The folder is unchanged.');
      else if (error instanceof Refused && error.refusal.refusal === 'folder_dialog_unavailable') { setNotice(name + ' could not show its folder dialog. Type the path below instead.'); setProblem(error); }
      else setProblem(error instanceof Error ? error : new Error(String(error)));
    } finally { working.current = false; setBusy(false); setDialogOpen(false); }
  };
  // A typed path is taken once the computer has opened it; one that is not there, or cannot be read, is said beside the field in the person's words.
  const go = async (machine: string, path: string) => {
    if (working.current) return;
    working.current = true; setBusy(true); setTypedWrong(''); setNotice('');
    try { const found = await foldersIn(machine, path); setProblem(null); choose(found.under); setOpen(false); }
    catch (error) {
      if (error instanceof Refused && NOT_A_FOLDER.includes(error.refusal.refusal)) setTypedWrong('There is no folder Lys can open at ' + path + ' on this computer. Check the path, or choose it in the dialog.');
      else setProblem(error instanceof Error ? error : new Error(String(error)));
    } finally { working.current = false; setBusy(false); }
  };
  // While a path is typed, the folders inside the folder it is being typed in are read once a pause in typing comes, to offer the names that match.
  const hinting = open && computer ? typingIn(typed) : null;
  const hintParent = hinting ? hinting.parent : null;
  useEffect(() => {
    if (!hintParent || !computer) return;
    let live = true;
    const timer = setTimeout(() => { foldersIn(computer, hintParent).then((found) => { if (live) setHints(found); }, () => { if (live) setHints(null); }); }, 250);
    return () => { live = false; clearTimeout(timer); };
  }, [hintParent, computer]);
  if (!computers.length) return <p className="folder-chooser">You choose its folder once a computer with Lys running on it is added.</p>;
  if (!open) return <p className="folder-chooser">
    <Act symbol="open" name={label ?? (chosen ? 'Choose another folder' : 'Choose a folder')} word={word} disabled={disabled} onClick={() => { setOpen(true); setTyped(chosen); setNotice(''); setTypedWrong(''); if (computer) void look(computer); }} />
  </p>;
  const offered = hinting && hints && hints.under === hinting.parent
    ? hints.folders.filter((entry) => !entry.startsWith('.') && entry.toLowerCase().startsWith(hinting.start.toLowerCase())).slice(0, 50).map((entry) => inside(hints.under, entry)) : [];
  const listId = 'folder-paths-' + (computer || 'none');
  return <div className="folder-chooser" role="group" aria-label="Choose a folder">
    {known ? null : <label className="field">Computer the folder is on<select name="folder-computer" value={computer} disabled={busy} onChange={(event) => { setPicking(event.target.value); setHints(null); setNotice(''); setTypedWrong(''); if (event.target.value) void look(event.target.value); }}>
      {computer ? null : <option value="">Choose a computer</option>}
      {computers.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
    </select></label>}
    {gone ? <div className="folder-recovery">
      <p>The folder chosen before, <code>{gone.folder}</code>, could not be read on {name}. Choose a readable folder, or ask its owner to check access.</p>
      <ErrorWords problem={gone.problem} />
    </div> : null}
    {computer ? <div className="folder-dialog">
      <Act symbol="open" name={'Choose the folder on ' + name} word={'Choose on ' + name} tone="primary" disabled={busy} onClick={() => { void dialog(computer); }} />
      <span className="hint">{dialogOpen ? 'The folder dialog is open on ' + name + '. Pick the folder there; this page waits for it.' : 'Opens ' + name + '’s own folder dialog on its screen.'}</span>
    </div> : null}
    {notice ? <p role="status" className="folder-notice">{notice}</p> : null}
    {computer ? <form className="folder-path" onSubmit={(event) => {
      // This form sits inside the page's own form on Add an agent; its submit must never reach that form, or Open would add the agent.
      event.preventDefault();
      event.stopPropagation();
      if (!typed.trim().startsWith('/')) { setTypedWrong('Type the whole path, starting with /, for example /Users/you/Developer/project.'); return; }
      void go(computer, typedPath(typed));
    }}>
      <label className="field">Or type the folder on {name}
        <input name="folder-path" value={typed} list={listId} onChange={(event) => { setTyped(event.target.value); setTypedWrong(''); }} placeholder="/Users/you/Developer/project" autoComplete="off" autoCapitalize="off" spellCheck={false} aria-describedby={typedWrong ? listId + '-wrong' : undefined} aria-invalid={typedWrong ? true : undefined} disabled={busy} />
      </label>
      <datalist id={listId}>{offered.map((path) => <option key={path} value={path} />)}</datalist>
      <Act symbol="enter" name="Open the folder typed" word="Open" type="submit" disabled={busy || !typed.trim()} />
      {typedWrong ? <p id={listId + '-wrong'} role="alert" className="why-not">{typedWrong}</p> : null}
    </form> : null}
    {problem ? <div className="folder-problem">
      <p>Lys could not ask {name || 'the computer'} about its folders. Check the computer’s runner, then try again.</p>
      <ErrorWords problem={problem} />
    </div> : null}
    <p><Act symbol="close" name="Cancel" word="Cancel" disabled={busy && dialogOpen} onClick={() => setOpen(false)} /></p>
  </div>;
}
