/** The organisation's context default as one agent inherits it: what it does in plain words, a way to give this agent levels of its own, and, for an administrator, the default itself to change for every agent. */
import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import { Refused, request } from '../../api';
import { ErrorWords } from '../people/Words';
import { Act } from '../../shell/Act';
import type { BudgetsView, ContextDefault, Limit } from './contract';

const ORGANISATION = '/budgets/organisation/organisation';

/** `20%`, `20% and 38%`, `20%, 30% and 38%`. */
function listed(levels: number[]): string {
  const words = levels.map((level) => level + '%');
  return words.length < 2 ? words.join('') : words.slice(0, -1).join(', ') + ' and ' + words[words.length - 1];
}

const at = (limits: Limit[], act: Limit['act']) => limits.filter((limit) => limit.act === act).map((limit) => limit.amount).sort((a, b) => a - b);

/** What the context levels do, in one sentence without its ending: `Warns at 20% and 38%, compacts at 40%`. */
export function contextWords(limits: Limit[]): string {
  const parts = [
    at(limits, 'notice').length ? 'warns at ' + listed(at(limits, 'notice')) : '',
    at(limits, 'tell').length ? 'tells its person at ' + listed(at(limits, 'tell')) : '',
    at(limits, 'compact').length ? 'compacts at ' + listed(at(limits, 'compact')) : '',
    at(limits, 'stop').length ? 'stops at ' + listed(at(limits, 'stop')) : '',
  ].filter((part) => part !== '');
  const sentence = parts.join(', ');
  return sentence.charAt(0).toUpperCase() + sentence.slice(1);
}

/** The default's sentence for this agent, in plain words. */
export function defaultWords(inherited: ContextDefault): string {
  if (!inherited.limits.length) return 'No context default is in force: an administrator switched it off, so this agent is not warned or compacted unless it has levels of its own.';
  const levels = contextWords(inherited.limits);
  if (inherited.set_aside) return 'This agent’s own context levels replace the default for every agent (' + levels.charAt(0).toLowerCase() + levels.slice(1) + ').';
  return levels + ' (the default for every agent).';
}

type Props = {
  inherited: ContextDefault;
  busy: boolean;
  /** Save the default's levels as this agent's own, beside its other limits, so they can be changed here. */
  adopt: (limits: Limit[]) => void;
  /** The budget table for the organisation's limits, given its view and how to say it changed. */
  editor: (view: BudgetsView, changed: (words: string) => void) => ReactNode;
};

export function InheritedContext({ inherited, busy, adopt, editor }: Props) {
  const [open, setOpen] = useState(false);
  const administrator = useOrganisation();
  const [notice, setNotice] = useState('');
  const [revision, setRevision] = useState(0);
  const organisation = useOrganisation(open ? revision : null);
  const own = !inherited.set_aside && inherited.limits.length > 0;
  return <div className="usage-default" aria-label="Context default">
    <p>{defaultWords(inherited)}</p>
    <div className="usage-default-acts">
      {own ? <Act symbol="edit" name="Change for this agent" word="Change for this agent" hint="Give this agent these levels as its own, to change in the table above; the default stays as it is for every other agent" disabled={busy} onClick={() => adopt(inherited.limits)} /> : null}
      {administrator.status === 'ok' ? <Act symbol={open ? 'close' : 'open'} name={open ? 'Close the default' : 'Change the default'} word={open ? 'Close the default' : 'Change the default'} hint="The context levels every agent without its own inherits" onClick={() => setOpen((value) => !value)} /> : null}
    </div>
    {administrator.status === 'failed' ? <><p className="why-not">Whether you may change the default could not be read.</p><ErrorWords problem={administrator.problem} /></> : null}
    {notice ? <p role="status">{notice}</p> : null}
    {open && organisation.status === 'ok' ? editor(organisation.view, (words) => { setNotice('The default for every agent: ' + words); setRevision((value) => value + 1); }) : null}
    {open && organisation.status === 'failed' ? <><p className="why-not">The default could not be read.</p><ErrorWords problem={organisation.problem} /></> : null}
  </div>;
}

type Read = { status: 'loading' } | { status: 'ok'; view: BudgetsView } | { status: 'refused' } | { status: 'failed'; problem: unknown };

/** The organisation's limits, which only an administrator reads: a refusal of `not_permitted` means the caller is not one, and any other failure is shown. `revision` null reads nothing. */
function useOrganisation(revision: number | null = 0): Read {
  const [read, setRead] = useState<Read>({ status: 'loading' });
  useEffect(() => {
    if (revision === null) return undefined;
    let live = true;
    setRead({ status: 'loading' });
    request<BudgetsView>(ORGANISATION).then(
      (view) => { if (live) setRead({ status: 'ok', view }); },
      (error: unknown) => {
        if (!live) return;
        setRead(error instanceof Refused && error.status === 403 ? { status: 'refused' } : { status: 'failed', problem: error });
      },
    );
    return () => { live = false; };
  }, [revision]);
  return read;
}
