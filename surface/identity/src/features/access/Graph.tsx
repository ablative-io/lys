/** The directory's responsibility links and the permission service's current reach answers. */
import { useParams } from 'react-router';
import { useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { readGrantWorld, nameOf } from '../grants/model';
import { resourcesSeen } from '../grants/CheckBox';
import { reachMap } from '../grants/check';
import './graph.css';

async function readGraph() {
  const world = await readGrantWorld();
  const reach = await reachMap([...resourcesSeen(world).values()]);
  return { world, reach };
}

export function Graph() {
  const { id } = useParams();
  const load = useLoad(readGraph, 'identity-graph');
  return <div className="page"><div className="eyebrow">Access</div><h1>Graph</h1>
    <p className="sub">Who answers to whom, and which resources the permission service says they can reach. Select an identity to follow its connections.</p>
    <Gate load={load} title="Graph" ok={({ world, reach }) => {
      const identities = [...world.who];
      const known = id === undefined || world.who.has(id);
      const shown = identities.filter(([key, person]) => !id || key === id || person.responsible === id || world.who.get(id)?.responsible === key);
      const edges = [...reach].flatMap(([resource, holders]) => [...holders].filter(([holder]) => !id || holder === id).map(([holder, actions]) => ({ resource, holder, actions })));
      return <>
        <div className="tabs"><a href="#/graph" aria-current={!id ? 'page' : undefined}>Everyone visible</a>
          {id && known ? <a href={'#/file/' + encodeURIComponent(id)}>Open {nameOf(world, id)}’s file</a> : null}
        </div>
        {!known ? <p role="status">Identity {id} is not in the directory records you may see.</p> : <>
          <div className="identity-graph">
            <section aria-label="People and responsibility"><h2>People and agents</h2>
              {shown.map(([key, person]) => <div className="graph-identity" key={key}>
                <a href={'#/graph/' + encodeURIComponent(key)} aria-current={key === id ? 'page' : undefined}>{person.name}</a>
                <span className="sec">{person.kind} · {person.state}</span>
                {person.responsible ? <span className="note">Answers to <a href={'#/graph/' + encodeURIComponent(person.responsible)}>{nameOf(world, person.responsible)}</a></span> : null}
              </div>)}
            </section>
            <section aria-label="Permission connections"><h2>Reaches</h2>
              {edges.map(({ resource, holder, actions }) => <div className="graph-connection" key={holder + ':' + resource}>
                <a href={'#/graph/' + encodeURIComponent(holder)}>{world.who.has(holder) ? nameOf(world, holder) : holder}</a>
                <span aria-hidden="true">→</span>
                <a href={'#/access/who/' + encodeURIComponent(resource)}>{resource}</a>
                <span className="note">{actions.join(', ')}</span>
              </div>)}
              {!edges.length ? <p className="note">No permitted connections were returned for the visible resources.</p> : null}
            </section>
          </div>
          <p className="note">Answers are limited to the directory and resources you may see. Select a resource to inspect the grant paths. These reads do not exercise a grant.</p>
        </>}
      </>;
    }} />
  </div>;
}
