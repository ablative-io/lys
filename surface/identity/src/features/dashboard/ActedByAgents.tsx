/**
 * What the person's agents registered and started for them (AGENTS-016 R5), and every act one of them took with a
 * grant a person may give it (AGENTS-019 R5), under one heading, newest first, each line in plain words: an act is
 * said in its shipped sentence, '<agent> used its grant for you, <time>: <sentence>.', the sentence as shipped so it reads whole: the record is the grant's use, which the check writes before the act runs, so the line never claims the act succeeded. A started line carries the session its launch
 * runs as and shows Stop, the ordinary Stop that ends that run and keeps the agent's certificate (AGENTS-017 R2).
 * Every line shows Withdraw, the emergency act behind the agent file's own confirmation (EmergencyStop): final, its
 * receipt saying the registrar cannot certify the agent again; an act's Withdraw is for the agent that acted. Nothing
 * here posts without a confirmation. No lines, no heading.
 */
import { useState } from 'react';
import { Act } from '../../shell/Act';
import { EmergencyStop, StopReceipt, WITHDRAW } from '../file/EmergencyStop';
import type { StopAnswer } from '../file/EmergencyStop';
import { SessionStop } from '../run/SessionStop';
import { clock } from '../file/time';
import type { ActedLine, PartRefused } from './contract';
import { refusedPart } from './contract';
import { Unread } from './Unread';

const said = (line: ActedLine, name: (id: string) => string): string =>
  line.kind === 'registered' || line.kind === 'started'
    ? `${name(line.acting_agent)} ${line.kind} ${name(line.agent)} for you, ${clock(line.at)}.`
    : `${name(line.acting_agent)} used its grant for you, ${clock(line.at)}: ${line.sentence ?? line.kind}.`;

function Line({ line, name, changed }: { line: ActedLine; name: (id: string) => string; changed: () => void }) {
  const [asking, setAsking] = useState(false);
  const [receipt, setReceipt] = useState<StopAnswer | null>(null);
  return <li data-acted-line={line.kind}>
    <span data-acted-words>{said(line, name)}</span>{' '}
    {line.kind === 'started' && line.session && !receipt ? <SessionStop agent={line.agent} session={line.session} name={name(line.agent)} where={null} /> : null}{' '}
    {receipt ? null : <Act symbol="suspend" name={`${WITHDRAW} ${name(line.agent)}`} word={WITHDRAW} tone="danger" data-act="withdraw" disabled={asking} onClick={() => setAsking(true)} />}
    <EmergencyStop id={line.agent} active open={asking} cancel={() => setAsking(false)}
      stopped={(answer) => { setAsking(false); setReceipt(answer); changed(); }} />
    {receipt ? <StopReceipt answer={receipt} /> : null}
  </li>;
}

export function ActedByAgents({ lines, names, changed }: { lines: ActedLine[] | PartRefused | undefined; names: Map<string, string>; changed: () => void }) {
  if (lines === undefined) return <span data-acted-ready hidden />;
  if (refusedPart(lines)) return <Unread part={lines} words="What your agents did for you could not be read." />;
  if (!lines.length) return <span data-acted-ready hidden />;
  const name = (id: string) => names.get(id) ?? id;
  return <section className="you-panel dash-acted" aria-label="Acted for you by your agents" data-acted-ready>
    <h2 className="section-h">Acted for you by your agents</h2>
    <ul>{lines.map((line) => <Line key={`${line.kind} ${line.agent} ${line.at} ${line.index ?? ''}`} line={line} name={name} changed={changed} />)}</ul>
  </section>;
}
