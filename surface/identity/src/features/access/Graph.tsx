/** The directory's responsibility links and the permission service's current reach answers, drawn as a graph. */
import { useLayoutEffect, useRef, useState } from 'react';
import { useParams } from 'react-router';
import { useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { readGrantWorld, nameOf } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import { resourcesSeen } from '../grants/CheckBox';
import { reachMap } from '../grants/check';
import { installedOf } from './installed';
import type { Installed } from './installed';
import './graph.css';

type Column = 'people' | 'agents' | 'resources';
interface GraphNode { id: string; title: string; detail: string; column: Column; href: string }
interface GraphEdge { id: string; from: string; to: string; label: string; kind: 'responsible' | 'installed' | 'reach' }
interface Drawn { nodes: GraphNode[]; edges: GraphEdge[] }
interface Line { id: string; path: string; kind: string; lit: boolean }

async function readGraph() {
  const world = await readGrantWorld();
  const [reach, installed] = await Promise.all([reachMap([...resourcesSeen(world).values()]), installedOf(world)]);
  return { world, reach, installed };
}

/** Every identity, app and resource the reads returned, and every recorded connection between them. */
export function drawnFrom(world: GrantWorld, reach: Map<string, Map<string, string[]>>, installed: Installed[]): Drawn {
  const nodes = new Map<string, GraphNode>();
  const edges: GraphEdge[] = [];
  const identity = (key: string): string => {
    const known = world.who.get(key);
    if (!nodes.has(key)) {
      nodes.set(key, {
        id: key, column: known?.kind === 'person' ? 'people' : 'agents',
        title: known ? known.name : key, detail: known ? known.kind + ' · ' + known.state : 'not in the directory you may see',
        href: '#/graph/' + encodeURIComponent(key),
      });
    }
    return key;
  };
  const resource = (key: string, href: string): string => {
    const id = 'resource:' + key;
    if (!nodes.has(id)) nodes.set(id, { id, column: 'resources', title: key, detail: 'resource', href });
    return id;
  };
  for (const [key, person] of world.who) {
    identity(key);
    if (person.responsible) {
      edges.push({ id: 'responsible:' + key, from: identity(person.responsible), to: key, label: nameOf(world, key) + ' answers to ' + nameOf(world, person.responsible), kind: 'responsible' });
    }
  }
  for (const { holder, name, target, words } of installed) {
    if (!world.who.has(holder) && !nodes.has(holder)) nodes.set(holder, { id: holder, column: 'agents', title: name, detail: 'app', href: '#/apps' });
    edges.push({ id: 'installed:' + holder + ':' + target, from: world.who.has(holder) ? identity(holder) : holder, to: resource(target, '#/directory/manage'), label: name + ' ' + words, kind: 'installed' });
  }
  for (const [key, holders] of reach) {
    for (const [holder, actions] of holders) {
      edges.push({ id: 'reach:' + holder + ':' + key, from: identity(holder), to: resource(key, '#/access/who/' + encodeURIComponent(key)), label: nameOf(world, holder) + ' may ' + actions.join(', ') + ' on ' + key, kind: 'reach' });
    }
  }
  return { nodes: [...nodes.values()], edges };
}

function Drawing({ drawn, focus }: { drawn: Drawn; focus: string | undefined }) {
  const [lines, setLines] = useState<Line[]>([]);
  const space = useRef<HTMLDivElement>(null);
  const placed = useRef(new Map<string, HTMLElement>());
  const near = new Set(focus ? drawn.edges.filter((edge) => edge.from === focus || edge.to === focus).flatMap((edge) => [edge.from, edge.to]) : []);
  useLayoutEffect(() => {
    const element = space.current;
    if (!element) return;
    const measure = () => {
      const origin = element.getBoundingClientRect();
      const next = drawn.edges.flatMap((edge) => {
        const from = placed.current.get(edge.from)?.getBoundingClientRect();
        const to = placed.current.get(edge.to)?.getBoundingClientRect();
        if (!from || !to) return [];
        const sameColumn = Math.abs(from.left - to.left) < 1;
        const x = (sameColumn ? from.left : from.right) - origin.left;
        const y = from.top + from.height / 2 - origin.top;
        const endX = to.left - origin.left;
        const endY = to.top + to.height / 2 - origin.top;
        const bend = sameColumn ? x - 60 : (x + endX) / 2;
        const path = sameColumn
          ? `M ${x} ${y} C ${bend} ${y}, ${bend} ${endY}, ${endX} ${endY}`
          : `M ${x} ${y} C ${bend} ${y}, ${bend} ${endY}, ${endX} ${endY}`;
        return [{ id: edge.id, kind: edge.kind, path, lit: !focus || edge.from === focus || edge.to === focus }];
      });
      setLines((previous) => JSON.stringify(previous) === JSON.stringify(next) ? previous : next);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    for (const node of placed.current.values()) observer.observe(node);
    measure();
    return () => observer.disconnect();
  }, [drawn, focus]);
  const headings: Record<Column, string> = { people: 'People', agents: 'Agents and apps', resources: 'Resources' };
  return <div className="permission-graph-scroll" role="region" aria-label="Permission graph" tabIndex={0}>
    <div className="permission-graph" ref={space}>
      <svg className="permission-graph-lines" aria-hidden="true">
        {lines.map((line) => <path key={line.id} d={line.path} data-kind={line.kind} data-lit={line.lit} />)}
      </svg>
      {(['people', 'agents', 'resources'] as const).map((column) => <section className="permission-graph-column" key={column} aria-label={headings[column]}>
        <h2>{headings[column]}</h2>
        {drawn.nodes.filter((node) => node.column === column).map((node) => <a
          className="permission-graph-node" key={node.id} href={node.href}
          aria-current={node.id === focus ? 'page' : undefined}
          data-dim={focus !== undefined && node.id !== focus && !near.has(node.id)}
          ref={(element) => { if (element) placed.current.set(node.id, element); else placed.current.delete(node.id); }}>
          <strong>{node.title}</strong><span className="note">{node.detail}</span>
        </a>)}
        {!drawn.nodes.some((node) => node.column === column) ? <p className="note">None visible.</p> : null}
      </section>)}
    </div>
  </div>;
}

export function Graph() {
  const { id } = useParams();
  const load = useLoad(readGraph, 'identity-graph');
  return <div className="page"><div className="eyebrow">Access</div><h1>Graph</h1>
    <p className="sub">Who answers to whom, and which resources the permission service says they can reach. Select anyone to follow their connections.</p>
    <Gate load={load} title="Graph" ok={({ world, reach, installed }) => {
      const known = id === undefined || world.who.has(id) || id.startsWith('app:');
      const drawn = drawnFrom(world, reach, installed);
      return <>
        <div className="tabs"><a href="#/graph" aria-current={!id ? 'page' : undefined}>Everyone visible</a>
          {id && world.who.has(id) ? <a href={'#/file/' + encodeURIComponent(id)}>Open {nameOf(world, id)}’s file</a> : null}
        </div>
        {!known ? <p role="status">Identity {id} is not in the directory records you may see.</p> : <>
          <Drawing drawn={drawn} focus={id} />
          <div className="permission-graph-key note">
            <span data-kind="responsible">answers to</span>
            <span data-kind="installed">recorded by the install</span>
            <span data-kind="reach">may reach, per the permission service</span>
          </div>
          <details className="permission-graph-list"><summary>Read the connections ({drawn.edges.length})</summary>
            <ul>{drawn.edges.map((edge) => <li key={edge.id} data-kind={edge.kind}>{edge.label}</li>)}</ul>
            {!drawn.edges.some((edge) => edge.kind === 'reach') ? <p className="note">No grant gives a permitted connection on the visible resources.</p> : null}
          </details>
          <p className="note">Answers are limited to the directory and resources you may see. Select a resource to inspect the grant paths. These reads do not exercise a grant.</p>
        </>}
      </>;
    }} />
  </div>;
}
