/**
 * An installer connects another computer without showing a code. Terminal users can ask for a code separately;
 * every installer or code replaces the previous one, and neither is kept in this tab's storage.
 */
import { useEffect, useRef, useState } from 'react';
import { Refused, operationId, request } from '../../api';
import { isRecord } from './contract';
import { Act } from '../../shell/Act';
import { problemWords } from '../people/Words';

/** A connection code as the service answers it, once. */
export interface JoinCode { machine: string; server: string; command: string; code: string }

export type InstallerKind = 'macos-pkg' | 'deb' | 'arch';
type InstallerGiven = { kind: 'installer'; platform: InstallerKind; filename: string };

/** A successful download or code uses the same receipt to ask the existing runner state again. */
export interface Connecting { machine: string; name: string; given: JoinCode | InstallerGiven | null; failure: string; at: 'row' | 'panel'; detail?: string }

export function readJoinCode(value: unknown, machine: string): JoinCode {
  if (!isRecord(value) || value.machine !== machine || typeof value.server !== 'string' || typeof value.command !== 'string'
    || typeof value.code !== 'string' || !value.code || !value.command.includes(value.server)) {
    throw new Refused(200, { refusal: 'JoinCodeUnreadable', reason: 'The answer did not carry a connection code for this computer.' });
  }
  return { machine, server: value.server, command: value.command, code: value.code };
}

/** Ask for a new connection code for `machine`, under an operation of its own: the answer is the one time it is shown. */
export async function askJoinCode(machine: string): Promise<JoinCode> {
  const answer = await request<unknown>('/network/machines/' + encodeURIComponent(machine) + '/join-code', { operation: operationId() });
  return readJoinCode(answer, machine);
}

/** Why a code was not given, in the plain words the add row and the panel both show. */
export const joinFailure = (error: unknown): string => error instanceof Refused && error.refusal.refusal === 'client_platform_unknown'
  ? 'Lys has no installer for this kind of computer yet. Ask the administrator to add one.' : problemWords(error);
/** The refusal itself, for the Error details fold. */
export const joinDetail = (error: unknown): string => error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : error instanceof Error ? error.message : String(error);

function installerFilename(disposition: string | null): string {
  const unreadable = () => new Refused(200, { refusal: 'InstallerFilenameUnreadable', reason: 'Lys did not give the installer a usable file name. Ask the administrator to check its downloads.' });
  if (!disposition) throw unreadable();
  let name: string;
  const extended = /(?:^|;)\s*filename\*\s*=\s*UTF-8'[^']*'([^;]*)/i.exec(disposition);
  if (extended) {
    try { name = decodeURIComponent(extended[1].trim()); }
    catch { throw unreadable(); }
  } else {
    if (/(?:^|;)\s*filename\*\s*=/i.test(disposition)) throw unreadable();
    const quoted = /(?:^|;)\s*filename\s*=\s*"((?:[^"\\]|\\.)*)"\s*(?:;|$)/i.exec(disposition);
    const plain = /(?:^|;)\s*filename\s*=\s*([^";\s][^;]*)/i.exec(disposition);
    name = quoted ? quoted[1].replace(/\\(.)/g, '$1') : plain?.[1].trim() ?? '';
  }
  if (!name.trim() || name.trim() === '.' || name.trim() === '..' || /[/\\\u0000-\u001f\u007f]/.test(name)) throw unreadable();
  return name;
}

const shellFilename = (name: string): string => {
  const file = name.startsWith('-') ? './' + name : name;
  return /^[A-Za-z0-9_./-]+$/.test(file) ? file : "'" + file.replace(/'/g, "'\"'\"'") + "'";
};

/** Copy `text` for the person, saying whether it was copied; when the browser refuses, they are told to copy it themselves. */
export function Copy({ text, what }: { text: string; what: string }) {
  const [said, setSaid] = useState('');
  const copy = async () => {
    try {
      if (!navigator.clipboard) throw new Error('this browser does not let the page copy');
      await navigator.clipboard.writeText(text);
      setSaid(what + ' is copied.');
    } catch (error) { setSaid(what + ' was not copied (' + (error instanceof Error ? error.message : String(error)) + '). Select it and copy it yourself.'); }
  };
  return <><Act symbol="copy" name={'Copy ' + what.toLowerCase()} onClick={() => void copy()} />{said ? <span role="status" className="sec"> {said}</span> : null}</>;
}

/** The command and the code, each with its Copy button, shown until the person is done with them. */
export function JoinShown({ name, given, kind, done }: { name: string; given: JoinCode; kind: InstallerKind | ''; done: () => void }) {
  return <div className="join-code" role="group" aria-label={'Connect ' + name}>
    <ClientDownload kind={kind} name={name} />
    <p>Run this on that computer. The code works once and is not shown again.</p>
    <p className="sec">The command asks for the code: paste it there.</p>
    <p><code data-join="command">{given.command}</code> <Copy text={given.command} what="The command" /></p>
    <p><code data-join="code">{given.code}</code> <Copy text={given.code} what="The code" /></p>
    <Act symbol="approve" name="Done" word="Done" onClick={done} />
  </div>;
}

type ClientRelease = { commit: string; files: Record<string, string> };
type Clients = { platforms: Record<string, ClientRelease> };

/** The builds of `lys` Lys holds for the kind of computer chosen above, by the platform names its clients list. */
export const platformsFor = (kind: InstallerKind | '', held: string[]): string[] =>
  kind === '' ? [] : held.filter((platform) => (kind === 'macos-pkg' ? platform.includes('apple-darwin') : platform.includes('linux')));

/** A platform's processor in words, as the person would know it for that kind of computer. */
const chipWords = (platform: string): string => {
  const mac = platform.includes('apple-darwin');
  if (platform.startsWith('aarch64')) return mac ? 'Apple silicon' : 'ARM';
  if (platform.startsWith('x86_64')) return mac ? 'Intel' : 'Intel or AMD';
  return platform;
};

/**
 * `lys` for the computer being connected, for the kind chosen above it: only the builds Lys holds for that kind, chosen
 * already when there is one. Nothing is guessed from this browser, which runs on the device in hand, not that computer.
 */
export function ClientDownload({ kind, name }: { kind: InstallerKind | ''; name: string }) {
  const [chosen, setChosen] = useState('');
  const [clients, setClients] = useState<Clients | null>(null);
  const [failure, setFailure] = useState('');
  useEffect(() => {
    let current = true;
    request<Clients>('/clients').then((answer) => {
      if (!isRecord(answer) || !isRecord(answer.platforms)) throw new Error('Lys answered, but did not say which downloads it has.');
      if (current) setClients(answer);
    }).catch((error: unknown) => { if (current) setFailure(joinFailure(error)); });
    return () => { current = false; };
  }, []);
  const offered = platformsFor(kind, Object.keys(clients?.platforms ?? {}));
  const platform = offered.length === 1 ? offered[0] : offered.includes(chosen) ? chosen : '';
  const release = clients?.platforms[platform];
  const family = kind === 'macos-pkg' ? 'Mac' : 'Linux computer';
  if (failure) return <div className="client-download"><p className="why-not">Lys could not read its downloads of lys. {failure}</p></div>;
  if (!clients) return null;
  if (kind === '') return <div className="client-download"><p>First, put lys on {name}: the command below runs it. Choose what kind of computer {name} is above, and its download of lys is offered here.</p></div>;
  if (!offered.length) return <div className="client-download"><p>Lys has no download of lys for a {family} yet, so this command cannot run on {name}. Use the installer above, or ask the administrator to add lys for a {family}.</p></div>;
  return <div className="client-download">
    <p>First, put lys on {name}: the command below runs it.</p>
    {offered.length > 1 ? <label>{name}’s processor <select aria-label={name + '’s processor'} value={platform} onChange={(event) => setChosen(event.target.value)}>
      <option value="">Choose its processor</option>{offered.map((each) => <option key={each} value={each}>{chipWords(each)}</option>)}
    </select></label> : null}
    {release ? <p><a href={'/api/clients/' + encodeURIComponent(platform) + '/lys'} download="lys">Download lys for {name} ({chipWords(platform)})</a>
{' · built from commit ' + release.commit + ' · '}<a href={'/api/clients/' + encodeURIComponent(platform) + '/lys-containment-entry'} download="lys-containment-entry">Download its companion</a>
      <span className="sec"> Keep both files together.</span></p> : null}
  </div>;
}

/** One installer is the default; the command and its one-time code stay inside an optional disclosure. */
export function ConnectingNow({ connecting, connect, renew, busy, done }: { connecting: Connecting; connect: (next: Connecting) => void; renew: () => void; busy: boolean; done: () => void }) {
  const downloaded = connecting.given && 'kind' in connecting.given ? connecting.given : null;
  const code = connecting.given && !('kind' in connecting.given) ? connecting.given : null;
  const [kind, setKind] = useState<InstallerKind | ''>(downloaded?.platform ?? '');
  const [terminal, setTerminal] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [failure, setFailure] = useState<unknown>(null);
  const working = useRef(false);
  const current = useRef(true);
  const address = useRef<string | null>(null);
  useEffect(() => {
    current.current = true;
    return () => { current.current = false; if (address.current) URL.revokeObjectURL(address.current); };
  }, []);
  const download = async () => {
    if (!kind || busy || working.current) return;
    working.current = true; setDownloading(true); setFailure(null);
    // A request can replace a code even if its file answer is lost.
    connect({ machine: connecting.machine, name: connecting.name, at: connecting.at, given: null, failure: '' });
    try {
      const answer = await request('/network/machines/' + encodeURIComponent(connecting.machine) + '/installer/' + kind, {}, 'POST', undefined, 'blob');
      if (!current.current) return;
      if (answer.headers.get('content-type')?.split(';')[0].trim().toLowerCase() === 'application/json') {
        throw new Refused(200, { refusal: 'InstallerDownloadUnreadable', reason: 'Lys sent an answer instead of an installer. Ask the administrator to check its downloads.' });
      }
      const filename = installerFilename(answer.headers.get('content-disposition'));
      if (!answer.blob.size) throw new Refused(200, { refusal: 'InstallerDownloadEmpty', reason: 'Lys sent an empty installer. Ask the administrator to check its downloads.' });
      const link = document.createElement('a');
      try {
        if (address.current) { URL.revokeObjectURL(address.current); address.current = null; }
        address.current = URL.createObjectURL(answer.blob);
        link.href = address.current; link.download = filename;
        document.body.append(link); link.click();
      } catch {
        if (address.current) { URL.revokeObjectURL(address.current); address.current = null; }
        throw new Refused(0, { refusal: 'InstallerSaveRefused', reason: 'This browser could not save the installer. Allow downloads for this page, then download it again.' });
      } finally { link.remove(); }
      setTerminal(false);
      connect({ machine: connecting.machine, name: connecting.name, at: connecting.at, given: { kind: 'installer', platform: kind, filename }, failure: '' });
    } catch (error) { if (current.current) setFailure(error); }
    finally { working.current = false; if (current.current) setDownloading(false); }
  };
  const filename = downloaded && downloaded.platform === kind ? downloaded.filename : null;
  return <div className="join-code" role="group" aria-label={'Connect ' + connecting.name}>
    <label className="field">What kind of computer is {connecting.name}?
      <select aria-label={'What kind of computer is ' + connecting.name + '?'} value={kind} disabled={busy || downloading} onChange={(event) => {
        const value = event.target.value;
        if (value === '' || value === 'macos-pkg' || value === 'deb' || value === 'arch') { setKind(value); setFailure(null); }
        else setFailure(new Refused(0, { refusal: 'client_platform_unknown', reason: 'Choose Mac, Debian or Ubuntu, or Arch.' }));
      }}>
        <option value="">Choose its kind</option><option value="macos-pkg">Mac</option><option value="deb">Linux: Debian or Ubuntu</option><option value="arch">Linux: Arch</option>
      </select>
    </label>
    <p className="sec">A Windows computer cannot run agents for Lys yet: there is no installer or lys for Windows.</p>
    <Act symbol="save" name={'Download the installer for ' + connecting.name} word={'Download the installer for ' + connecting.name} tone="primary" disabled={!kind || busy || downloading} onClick={() => void download()} />
    {downloading ? <p role="status">Making the installer…</p> : null}
    {failure ? <div className="why-not" role="alert"><p>{joinFailure(failure)}</p><details><summary>Error details</summary><p className="refusal-name" style={{ overflowWrap: 'anywhere' }}>{joinDetail(failure)}</p></details></div> : null}
    {filename ? <p role="status">Your browser was asked to save {filename}. Open the latest file.</p> : null}
    {kind === 'macos-pkg' ? <p>Open the file. If your Mac says it cannot check the developer or that the file was not opened, open System Settings, choose Privacy &amp; Security, scroll down to the message about this installer and choose Open Anyway, then open the file again.</p>
      : kind === 'deb' ? <><p>Open the file and choose Install. If nothing offers to install it, open a terminal in the folder it was saved to and run:</p><p><code data-installer="command">sudo apt install ./{filename ? shellFilename(filename).replace(/^\.\//, '') : '<file>'}</code></p></>
      : kind === 'arch' ? <p><code data-installer="command">sudo pacman -U {filename ? shellFilename(filename) : '<file>'}</code></p> : null}
    <p>This computer appears here as Up once it connects.</p>
    <p className="sec">A new installer or terminal code replaces any earlier one. Open the latest file.</p>
    <details data-connect="terminal" open={terminal}>
      <summary onClick={(event) => { event.preventDefault(); setTerminal((open) => !open); }}>Connect from a terminal instead</summary>
      {terminal ? <>
        <p className="sec">A new code replaces any code given before, and the earlier one stops working. It also replaces any installer downloaded before it.</p>
        {code ? <JoinShown name={connecting.name} given={code} kind={kind} done={done} /> : null}
        {connecting.failure ? <><p className="why-not" role="alert">Lys could not give a connection code for {connecting.name}. If one was given, it is not shown again: a new code replaces it.</p><p>{connecting.failure}</p>{connecting.detail ? <details><summary>Error details</summary><p className="refusal-name" style={{ overflowWrap: 'anywhere' }}>{connecting.detail}</p></details> : null}</> : null}
        <Act symbol="again" name="Get a new connection code" word="New code" disabled={busy || downloading} onClick={() => { setFailure(null); renew(); }} />
      </> : null}
    </details>
    {terminal && code ? null : <Act symbol="approve" name="Done" word="Done" disabled={busy || downloading} onClick={done} />}
  </div>;
}

/** Ask for a new code for `machine`, telling `connect` how it went. */
export function useNewCode(machine: string, name: string, at: Connecting['at'], connect: (next: Connecting) => void) {
  const [busy, setBusy] = useState(false);
  const working = useRef(false);
  const renew = async () => {
    if (working.current) return;
    working.current = true; setBusy(true);
    connect({ machine, name, given: null, failure: '', at });
    try { connect({ machine, name, given: await askJoinCode(machine), failure: '', at }); }
    catch (error) { connect({ machine, name, given: null, failure: joinFailure(error), detail: joinDetail(error), at }); }
    finally { working.current = false; setBusy(false); }
  };
  return { busy, renew: () => void renew() };
}

/** The installer chooser opens below the computers just added. */
export function ConnectRow({ connecting, connect, done, width }: { connecting: Connecting; connect: (next: Connecting) => void; done: () => void; width: number }) {
  const { busy, renew } = useNewCode(connecting.machine, connecting.name, 'row', connect);
  return <tr className="listing-add" data-add="connect"><td colSpan={width}><ConnectingNow connecting={connecting} connect={connect} renew={renew} busy={busy} done={done} /></td></tr>;
}

/** The same installer and terminal choices reconnect a computer from its detail panel. */
export function ConnectComputer({ machine, name, connecting, connect, done }: { machine: string; name: string; connecting: Connecting | null; connect: (next: Connecting) => void; done: () => void }) {
  const { busy, renew } = useNewCode(machine, name, 'panel', connect);
  return <div className="join-new">
    <ConnectingNow connecting={connecting ?? { machine, name, given: null, failure: '', at: 'panel' }} connect={connect} renew={renew} busy={busy} done={done} />
  </div>;
}
