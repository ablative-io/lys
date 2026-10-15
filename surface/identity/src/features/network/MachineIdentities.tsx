/**
 * A computer's machines (ACCESS-005 R2): each join with a connection code made one, from the key that computer sent,
 * answering to the administrator who gave the code. Each is shown with its key id, the person answering for it, the
 * grants it holds and what each is on, and the administrator may give it part of a grant they hold, through the same
 * grant request every other giving uses. A machine never gives anything on, so nothing here offers it that.
 */
import { useState } from 'react';
import { Refused, api, operationId, request, useLoad } from '../../api';
import type { DelegateBody, Grant, GrantModel } from '../../generated/grants';
import { actionWords, resourceWords } from '../grants/action-words';
import { ErrorWords, STATUS } from '../people/Words';
import { withinPassOn } from '../grants/model';
import { day } from '../file/time';
import { Act } from '../../shell/Act';
import { readMachineIdentities } from './contract';
import type { MachineGrant, MachineIdentity } from './contract';
import { RecordCopy } from '../../shell/RecordCopy';
import { RecordName } from '../../shell/RecordName';
import { TableScroll } from '../../shell/Listing';

/** Every machine the service knows, for the administrator. */
export async function readIdentities(): Promise<MachineIdentity[]> {
  return readMachineIdentities(await request<unknown>('/network/machine-identities')).machines;
}

const whoHref = (grant: MachineGrant) => '#/access/who/' + encodeURIComponent(grant.resource.kind + ':' + grant.resource.id);

const MODES: Record<MachineGrant['mode'], string> = { outright: 'At once', by_draft: 'By draft', by_two: 'By two approvals' };

/** Every machine a computer's joins made, the one it answers as now first. */
export function MachineHolders({ machine, name, identities, me, changed }: { machine: string; name?: string; identities: MachineIdentity[]; me: string; changed: (message: string) => void }) {
  const mine = identities.filter((identity) => identity.machine === machine).reverse();
  if (!mine.length) return <p className="dim">This computer has not joined with a connection code, so it is no machine yet and holds nothing.</p>;
  return <>{mine.map((identity) => <MachineCard key={identity.identity} identity={identity} name={name} me={me} changed={changed} />)}</>;
}

function MachineCard({ identity, name, me, changed }: { identity: MachineIdentity; name?: string; me: string; changed: (message: string) => void }) {
  const active = !identity.replaced && identity.state === 'active';
  return <div className="card" aria-label={'Machine ' + identity.identity} data-machine={identity.identity}>
    <div className="row"><span className="sec">Machine</span><RecordCopy name={name ?? 'Machine identity'} value={identity.identity} /></div>
    <div className="row"><span className="sec">Key id</span><span data-key title={identity.key}><RecordCopy name="Signing key" value={identity.key} /></span></div>
    <div className="row"><span className="sec">Answers to</span><span><a href={'#/file/' + identity.responsible}>{identity.responsible_name ?? 'a person outside your view'}</a>, who gave its connection code</span></div>
    <div className="row"><span className="sec">Joined</span><span>{day(identity.joined_at)}</span></div>
    <div className="row"><span className="sec">Stands</span><span>{identity.replaced ? 'Replaced by a later join of this computer; its grants no longer count.' : identity.state === 'active' ? 'Active' : (STATUS[identity.state] ?? 'Not active') + '; its grants do not count while it is not active.'}</span></div>
    <TableScroll label="Machine grants"><table className="usage-table" aria-label={'Grants held by ' + identity.identity}><thead><tr><th>On</th><th>Actions</th><th>How</th><th>Grant</th></tr></thead><tbody>
      {identity.grants.map((grant) => <tr key={grant.grant} data-grant={grant.grant}>
        <td><a href={whoHref(grant)}>{resourceWords(grant.resource)}</a></td>
        <td className="mono">{grant.actions.join(', ')}</td>
        <td>{MODES[grant.mode]}{grant.admitted ? '' : ', not in force now'}</td>
        <td><RecordName id={grant.grant} name={'Access to ' + resourceWords(grant.resource)} /></td>
      </tr>)}
      {identity.grants.length ? null : <tr><td colSpan={4} className="dim">It holds no grant.</td></tr>}
    </tbody></table></TableScroll>
    {active ? <GiveToMachine identity={identity} name={name ?? 'this machine'} me={me} changed={changed} /> : null}
  </div>;
}

type Giving = { at: 'closed' } | { at: 'open' } | { at: 'sending' } | { at: 'refused'; refused: Refused };

/** The grants `me` holds that may be passed on to a machine, and still stand. */
const passableToMachines = (grants: Grant[], me: string): Grant[] => grants.filter((grant) => grant.holder === me && !grant.revoked
  && grant.standing.stands && grant.pass_on.kind === 'to' && grant.pass_on.recipients.includes('machine'));

/** Give a machine part of a grant the administrator holds: the same `POST /grants` every giving sends, answering to the machine's own person. */
function GiveToMachine({ identity, name, me, changed }: { identity: MachineIdentity; name: string; me: string; changed: (message: string) => void }) {
  const [giving, setGiving] = useState<Giving>({ at: 'closed' });
  const [picked, setPicked] = useState('');
  const [relation, setRelation] = useState('');
  const open = giving.at !== 'closed';
  // Your grants are read only once the form is opened, and once while it stays open.
  const load = useLoad(async (): Promise<{ sources: Grant[]; model: GrantModel } | null> => {
    if (!open) return null;
    const [list, model] = await Promise.all([api.grants(), api.model()]);
    return { sources: passableToMachines(list.grants, me), model };
  }, 'machine-give:' + identity.identity + ':' + String(open));
  const close = <Act symbol="close" name="Close" word="Close" onClick={() => setGiving({ at: 'closed' })} />;
  if (!open) return <Act symbol="add" name={'Give ' + name + ' a grant'} word="Give a grant" onClick={() => setGiving({ at: 'open' })} />;
  if (load.status === 'refused') return <><p className="why-not">Your grants could not be read, so none can be given now.</p><ErrorWords problem={load.refused} />{close}</>;
  if (load.status === 'loading' || load.data === null) return <p className="dim">Reading the grants you may pass on…</p>;
  const { sources, model } = load.data;
  if (!sources.length) return <><p className="dim">You hold no grant that may be passed on to a machine. A grant may be given to machines only when it says so.</p>{close}</>;
  const source = sources.find((grant) => grant.id === picked) ?? sources[0];
  const relations = Object.entries(model.relations).filter(([, actions]) => withinPassOn(source, actions)).map(([name]) => name).sort();
  const chosen = relations.includes(relation) ? relation : relations[0] ?? '';
  const give = async () => {
    if (giving.at === 'sending' || !chosen) return;
    setGiving({ at: 'sending' });
    const body: DelegateBody = {
      operation: operationId(), route: 'browser', source: source.id, recipient: identity.identity, responsible: identity.responsible,
      resource: source.resource, relation: chosen, pass_on: { kind: 'use_only' }, window: { starts_at: Math.floor(Date.now() / 1000), ends_at: source.window.ends_at },
    };
    try {
      const answer = await api.delegate(body);
      if (answer.operation !== body.operation || typeof answer.grant !== 'string' || !/^grant-[0-9a-f]{32}$/.test(answer.grant)) {
        throw new Refused(200, { refusal: 'UnconfirmedAnswer', reason: 'The service did not confirm this grant. Reload this page to see whether the machine holds it.' });
      }
      setGiving({ at: 'closed' });
      changed('Given. ' + name + ' now holds ' + chosen + ' on ' + resourceWords(source.resource) + ', answering to ' + (identity.responsible_name ?? 'its person') + '.');
    } catch (error) {
      setGiving({ at: 'refused', refused: error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) }) });
    }
  };
  return <form className="usage-add-row" aria-label={'Give ' + name + ' a grant'} onSubmit={(event) => { event.preventDefault(); void give(); }}>
    <label className="field">From your grant<select name="source" aria-label="From the grant" value={source.id} disabled={giving.at === 'sending'} onChange={(event) => setPicked(event.target.value)}>
      {sources.map((grant) => <option key={grant.id} value={grant.id}>{grant.relation} on {resourceWords(grant.resource)}</option>)}
    </select></label>
    <label className="field">What it may do<select name="relation" aria-label="Relation to give" value={chosen} disabled={giving.at === 'sending'} onChange={(event) => setRelation(event.target.value)}>
      {relations.map((each) => <option key={each} value={each}>{each}: {actionWords(model, source.resource, model.relations[each] ?? [])}</option>)}
    </select></label>
    <span><Act symbol="send" name="Give" word="Give" type="submit" tone="primary" disabled={giving.at === 'sending' || !chosen} />{' '}
      <Act symbol="close" name="Cancel" word="Cancel" disabled={giving.at === 'sending'} onClick={() => setGiving({ at: 'closed' })} /></span>
    {giving.at === 'refused' ? <ErrorWords problem={giving.refused} /> : null}
  </form>;
}
