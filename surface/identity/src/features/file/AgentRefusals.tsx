/** The tool calls an agent was refused, newest first, with the runners whose refusals have been read. */
import { Refused, request, useLoad } from '../../api';
import { readableMachine } from '../network/contract';
import type { NetworkView } from '../network/contract';
import { DirectoryGate as Gate, ErrorWords } from '../people/Words';
import type { RefusalRecord, RefusalsView } from './policyContract';
import { clockMs } from './time';

const when = clockMs;

function lift(refusal: RefusalRecord): string {
  if (!refusal.grantable) return 'not grantable';
  if (!refusal.permission) return 'grantable';
  const { resource, action } = refusal.permission;
  return 'grantable by a grant of ' + action + ' on ' + resource.kind + ' ' + resource.id + (refusal.grantor ? ', from ' + refusal.grantor : '');
}

function Coverage({ readFrom }: { readFrom: string[] }) {
  const computers = useLoad(async () => {
    const network = await request<NetworkView>('/network');
    if (!network || !Array.isArray(network.machines) || network.machines.some((machine) => !readableMachine(machine) || !machine.name.trim())) {
      throw new Refused(0, { refusal: 'AgentComputerUnreadable', reason: 'The service did not answer valid computer names and retirement states.' });
    }
    return network.machines;
  }, 'agent-refusal-computers');
  if (computers.status === 'loading') return <p className="note" data-coverage>Reading computer names for the runners whose refusals were read…</p>;
  if (computers.status === 'refused') return <div className="note" data-coverage>
    <p>Computer names could not be read, so the computers covered by this list are not confirmed.</p>
    <ErrorWords problem={computers.refused} />
  </div>;
  const read = [...new Set(readFrom)].map((id) => computers.data.find((machine) => machine.id === id));
  const names = read.flatMap((machine) => machine?.state === 'in_use' ? [machine.name] : []);
  const unknown = read.filter((machine) => !machine).length;
  return <p className="note" data-coverage>
    {names.length ? 'Read from runners on ' + names.join(', ') + '. Refusals on any other runner are not read here.'
      : 'None of the runners read could be matched to a current computer. This list does not confirm coverage of any current computer.'}
    {unknown ? ' ' + unknown + (unknown === 1 ? ' runner is' : ' runners are') + ' not listed in the computer network, so ' + (unknown === 1 ? 'its name and retirement state are' : 'their names and retirement states are') + ' unknown.' : null}
  </p>;
}

export function AgentRefusals({ id }: { id: string }) {
  const load = useLoad(async () => {
    const view = await request<RefusalsView>('/agents/' + encodeURIComponent(id) + '/refusals');
    if (view.agent !== id || !Array.isArray(view.refusals) || !Array.isArray(view.read_from) || view.read_from.some((source) => typeof source !== 'string' || !source)) throw new Error('The refusals answer did not name this agent and the runners read.');
    return view;
  }, 'agent-refusals:' + id);
  return (
    <>
      <div className="head"><div><h2>Refused tool calls</h2><p>What this agent's policy stopped, newest first.</p></div></div>
      <Gate load={load} title="Refused tool calls" ok={(view) => (
        <section className="card">
          {view.read_from.length ? <Coverage readFrom={view.read_from} />
            : <p className="note" data-coverage>No runner’s refusals have been read yet, so this list may be incomplete.</p>}
          {view.refusals.length ? view.refusals.map((refusal) => (
            <article className="row" key={refusal.source + ':' + refusal.attempt} data-refusal>
              <div>
                <div><strong>{refusal.tool}</strong> <span className="mono">{refusal.target}</span></div>
                <div className="note">{refusal.words}</div>
                <div className="note">Rule {refusal.rule ?? 'none'} · check {refusal.check} · policy version {refusal.policy_version} · {lift(refusal)}</div>
              </div>
              <span className="dim">{when(refusal.at)}</span>
            </article>
          )) : <p>No refused tool calls have been read.</p>}
        </section>
      )} />
    </>
  );
}
