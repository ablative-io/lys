/**
 * The Start drawer on an agent's file (DIRECTORY-029 R13): the one control
 * that works in step 1. The directory checks the agent before it gives a
 * command, keeps a launch record for every command it gives, and never runs
 * one: the command is text to copy into the machine's shell.
 *
 * Only the administrator the directory admits has a working Start in step
 * 1; anyone else sees the refusal naming the agent and the right they lack.
 * The drawer names the five checks in the order the directory runs them,
 * and shows each result in the directory's own words. Only when every check
 * passes does it show the command, its working directory, a Copy control and
 * the launch record's facts. Copying sends nothing and changes no state.
 */
import { useState } from 'react';
import { useParams } from 'react-router';
import { api, useLoad } from '../../api';
import { START_CHECKS } from '../../generated';
import type { LaunchStateView, StartCheck, StartGiven, StartRefused } from '../../generated';
import { Gate } from '../signin/Gate';
import { StartNotice, ask } from './StartNotice';

/** The Start drawer of the agent the address names. */
export function StartPage() {
  const { id = '' } = useParams();
  return <StartDrawer agent={id} />;
}

/** The agent file's Start control, and the drawer it opens. */
export function StartDrawer({ agent }: { agent: string }) {
  const load = useLoad(() => api.people(), 'start-scope:' + agent);
  return (
    <div className="page">
      <div className="eyebrow">
        <a href={'#/file/' + encodeURIComponent(agent)}>Agent file</a> / Start
      </div>
      <h1>Start {agent}</h1>
      <Gate load={load} title="Start" ok={(people) => (people.scope === 'directory' ? <Drawer agent={agent} /> : <NoRight agent={agent} />)} />
    </div>
  );
}

function NoRight({ agent }: { agent: string }) {
  return (
    <section className="card">
      <button className="btn primary" data-act="start" disabled>Start</button>
      <p className="why-not" role="note">
        You lack the right to start this agent, {agent}: only its responsible person or a directory administrator may
        start it.
      </p>
    </section>
  );
}

type Answer = { given: StartGiven } | { refused: StartRefused } | { failed: string } | null;

function Drawer({ agent }: { agent: string }) {
  const [open, setOpen] = useState(false);
  const [profileVersion, setProfileVersion] = useState('');
  const [machine, setMachine] = useState('');
  const [answer, setAnswer] = useState<Answer>(null);
  const [copied, setCopied] = useState('');
  const submit = async () => {
    setCopied('');
    const asked = await ask<StartGiven>('POST', '/agents/' + encodeURIComponent(agent) + '/start', { profile_version: profileVersion, machine });
    if (asked.ok) setAnswer({ given: asked.body });
    else if (Array.isArray((asked.body as Partial<StartRefused> | null)?.refused)) setAnswer({ refused: asked.body as StartRefused });
    else setAnswer({ failed: asked.words });
  };
  const checks: StartCheck[] | undefined = answer && 'given' in answer ? answer.given.checks : answer && 'refused' in answer ? answer.refused.checks : undefined;
  return (
    <section className="card" data-drawer="start">
      <button className="btn primary" data-act="start" onClick={() => setOpen(true)}>Start</button>
      {open ? (
        <div className="drawer">
          <h2>Start command</h2>
          <p className="sub">The directory checks this agent, keeps a launch record and gives you its command. You run it on the machine; the agent shows as running only when its own signed report names the launch record.</p>
          <label>
            Profile version <input name="profile_version" value={profileVersion} onChange={(event) => setProfileVersion(event.target.value)} />
          </label>
          <label>
            Machine, from the role's machines <input name="machine" value={machine} onChange={(event) => setMachine(event.target.value)} />
          </label>
          <button className="btn primary" data-act="ask-start" onClick={() => void submit()}>Check and give the command</button>
          <ol data-checks>
            {START_CHECKS.map((name) => {
              const result = checks?.find((check) => check.check === name);
              return (
                <li key={name} data-check={name}>
                  <b>{name}</b>: {result ? result.words : 'checked when you ask'}
                </li>
              );
            })}
          </ol>
          {answer && 'refused' in answer ? (
            <div className="why-not" role="alert">
              {answer.refused.refused.map((refusal) => (
                <p key={refusal.refusal + refusal.words}>
                  <b>{refusal.refusal}</b> {refusal.words}
                </p>
              ))}
            </div>
          ) : null}
          {answer && 'failed' in answer ? <p className="why-not" role="alert">{answer.failed}</p> : null}
          {answer && 'given' in answer ? <GivenCommand given={answer.given} copied={copied} setCopied={setCopied} /> : null}
        </div>
      ) : null}
    </section>
  );
}

function GivenCommand({ given, copied, setCopied }: { given: StartGiven; copied: string; setCopied: (words: string) => void }) {
  const record = given.launch_record;
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(given.command);
      setCopied('Copied. Copying changes nothing: the start stays unconfirmed until its report arrives.');
    } catch (error) {
      setCopied('The command could not be copied: ' + String(error));
    }
  };
  const initial: LaunchStateView = { launch_record: record.id, agent: record.agent, state: given.state, words: given.words, session: null, withdrawal: null };
  return (
    <div data-given>
      <pre data-command>{given.command}</pre>
      <p>
        Working directory: <code data-working-directory>{given.working_directory}</code>
      </p>
      <button className="btn" data-act="copy" onClick={() => void copy()}>Copy</button>
      {copied ? <p className="dim" role="status">{copied}</p> : null}
      <dl data-launch-record>
        <dt>Launch record</dt><dd>{record.id}</dd>
        <dt>Machine</dt><dd>{record.machine}</dd>
        <dt>Runs</dt><dd>{record.executable} in {record.working_directory}</dd>
        <dt>Profile version</dt><dd>{record.profile_version}</dd>
        <dt>Credential ids</dt><dd>{record.credential_ids.join(', ')}</dd>
        <dt>State</dt><dd>{given.state}</dd>
      </dl>
      <StartNotice initial={initial} />
    </div>
  );
}
