/** Every person, agent and resource, and the relations between them, laid out by a small force simulation. */
import { useMemo, useRef, useState } from 'react';
import type { PointerEvent, WheelEvent } from 'react';
import { useNavigate, useParams } from 'react-router';
import { useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { readGrantWorld, nameOf } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import { resourcesSeen } from '../grants/CheckBox';
import { reachMap } from '../grants/check';
import { installedOf } from './installed';
import type { Installed } from './installed';
import './graph.css';

type Kind = 'person' | 'agent' | 'app' | 'resource';
interface Node { id: string; label: string; kind: Kind; active: boolean; x: number; y: number; vx: number; vy: number }
interface Edge { a: string; b: string; kind: 'grant' | 'void' | 'answers' | 'installed'; label: string }
type Show = { grants: boolean; answers: boolean; installed: boolean };

const W = 1400;
const H = 820;

async function readGraph() {
  const world = await readGrantWorld();
  const [reach, installed] = await Promise.all([reachMap([...resourcesSeen(world).values()]), installedOf(world)]);
  return { world, reach, installed };
}

function model(world: GrantWorld, installed: Installed[], show: Show): { nodes: Node[]; edges: Edge[] } {
  const nodes = new Map<string, Node>();
  const add = (id: string, label: string, kind: Kind, active: boolean) => {
    if (!nodes.has(id)) nodes.set(id, { id, label, kind, active, x: 0, y: 0, vx: 0, vy: 0 });
  };
  for (const [id, who] of world.who) add(id, who.name, who.kind, who.state === 'active');
  const edges: Edge[] = [];
  for (const grant of world.list.grants) {
    if (!world.who.has(grant.holder) && !show.installed) continue;
    add(grant.holder, nameOf(world, grant.holder), world.who.has(grant.holder) ? 'agent' : 'app', true);
    const kind = grant.resource.kind.toLowerCase(), id = grant.resource.id.toLowerCase();
    const members = world.who.has(grant.resource.id) ? [grant.resource.id]
      : kind === 'agents' || id === 'agents' || (kind === 'agent' && id === '*') ? [...world.who].filter(([, w]) => w.kind === 'agent').map(([k]) => k)
      : kind === 'people' || id === 'people' || (kind === 'person' && id === '*') ? [...world.who].filter(([, w]) => w.kind === 'person').map(([k]) => k)
      : [];
    const targets = members.length ? members.filter((m) => m !== grant.holder) : ['resource:' + grant.resource.kind + ':' + grant.resource.id];
    if (!members.length) add(targets[0], grant.resource.id, 'resource', true);
    if (show.grants) {
      for (const target of targets) edges.push({ a: grant.holder, b: target, kind: grant.standing.stands ? 'grant' : 'void',
        label: nameOf(world, grant.holder) + ' may ' + grant.actions.join(', ') + ' on ' + (members.length ? nameOf(world, target) : grant.resource.id) + (grant.standing.stands ? '' : ' (does not stand)') });
    }
  }
  if (show.answers) {
    for (const [id, who] of world.who) if (who.responsible) edges.push({ a: id, b: who.responsible, kind: 'answers', label: who.name + ' answers to ' + nameOf(world, who.responsible) });
  }
  for (const entry of installed) {
    add('resource:directory', 'directory', 'resource', true);
    if (!world.who.has(entry.holder)) add(entry.holder, entry.name, 'app', true);
    if (show.installed) edges.push({ a: entry.holder, b: 'resource:directory', kind: 'installed', label: entry.name + ' ' + entry.words });
  }
  return { nodes: [...nodes.values()], edges };
}

function layout(nodes: Node[], edges: Edge[]) {
  let seed = 7;
  const rnd = () => (seed = (seed * 9301 + 49297) % 233280) / 233280;
  const at = new Map(nodes.map((node) => [node.id, node]));
  const column = (node: Node) => node.kind === 'person' ? W * 0.14 : node.kind === 'agent' ? W * 0.42 : node.kind === 'app' ? W * 0.9 : W * 0.72;
  for (const node of nodes) { node.x = column(node) + (rnd() - 0.5) * 80; node.y = H / 2 + (rnd() - 0.5) * H * 0.8; }
  for (let it = 0; it < 420; it++) {
    const k = 1 - it / 420;
    for (let i = 0; i < nodes.length; i++) for (let j = i + 1; j < nodes.length; j++) {
      const a = nodes[i], b = nodes[j];
      let dx = a.x - b.x, dy = a.y - b.y;
      const d2 = dx * dx + dy * dy + 0.01, f = 9000 / d2, d = Math.sqrt(d2);
      dx /= d; dy /= d; a.vx += dx * f; a.vy += dy * f; b.vx -= dx * f; b.vy -= dy * f;
    }
    for (const edge of edges) {
      const a = at.get(edge.a), b = at.get(edge.b);
      if (!a || !b) continue;
      const dx = b.x - a.x, dy = b.y - a.y, d = Math.sqrt(dx * dx + dy * dy) || 1, f = (d - 200) * 0.0008 * d;
      a.vx += dx / d * f; a.vy += dy / d * f; b.vx -= dx / d * f; b.vy -= dy / d * f;
    }
    for (const node of nodes) {
      node.vx += (column(node) - node.x) * 0.08; node.vy += (H / 2 - node.y) * 0.003;
      node.x += Math.max(-12, Math.min(12, node.vx * k)); node.y += Math.max(-12, Math.min(12, node.vy * k));
      node.vx *= 0.6; node.vy *= 0.6;
      node.x = Math.max(40, Math.min(W - 40, node.x)); node.y = Math.max(30, Math.min(H - 30, node.y));
    }
  }
}

const colour = (kind: Kind): string => kind === 'person' ? 'var(--accent)' : kind === 'agent' ? '#e8c9a6' : kind === 'app' ? '#8b8b96' : '#5f7f9a';

function Shape({ node }: { node: Node }) {
  const stroke = colour(node.kind);
  if (node.kind === 'resource') return <rect x={-6} y={-6} width={12} height={12} rx={2} transform="rotate(45)" fill="var(--surface-card)" stroke={stroke} strokeWidth={1.5} />;
  if (node.kind === 'app') return <rect x={-9} y={-9} width={18} height={18} rx={4} fill="var(--surface-card)" stroke={stroke} strokeWidth={1.5} />;
  return <circle r={node.kind === 'person' ? 9 : 7} fill={node.active ? stroke : 'var(--surface-card)'} stroke={stroke} strokeWidth={1.5} />;
}

function Drawing({ world, reach, installed, focus }: { world: GrantWorld; reach: Map<string, Map<string, string[]>>; installed: Installed[]; focus: string | undefined }) {
  const navigate = useNavigate();
  const [show, setShow] = useState<Show>({ grants: true, answers: true, installed: true });
  const { nodes, edges } = useMemo(() => { const m = model(world, installed, show); layout(m.nodes, m.edges); return m; }, [world, installed, show]);
  const at = new Map(nodes.map((node) => [node.id, node]));
  const hot = focus ? new Set([focus, ...edges.filter((edge) => edge.a === focus || edge.b === focus).flatMap((edge) => [edge.a, edge.b])]) : null;
  const picked = focus ? at.get(focus) : undefined;
  const reaches = focus ? [...reach].flatMap(([resource, holders]) => { const actions = holders.get(focus); return actions ? [{ resource, actions }] : []; }) : [];
  const reachedBy = picked?.kind === 'resource' ? [...(reach.get(picked.label) ?? new Map<string, string[]>())] : [];
  const go = (id: string) => navigate('/graph/' + encodeURIComponent(id));
  const [view, setView] = useState({ x: 0, y: 0, w: W, h: H });
  const drag = useRef<{ x: number; y: number; moved: boolean } | null>(null);
  const svg = useRef<SVGSVGElement>(null);
  const scale = () => { const box = svg.current?.getBoundingClientRect(); return box && box.width ? view.w / box.width : 1; };
  const zoom = (factor: number, cx = view.x + view.w / 2, cy = view.y + view.h / 2) => setView((v) => {
    const w = Math.min(W * 1.6, Math.max(W / 6, v.w * factor)); const k = w / v.w;
    return { x: cx - (cx - v.x) * k, y: cy - (cy - v.y) * k, w, h: v.h * k };
  });
  const onWheel = (event: WheelEvent<SVGSVGElement>) => {
    const box = svg.current?.getBoundingClientRect();
    if (!box) return;
    const cx = view.x + (event.clientX - box.left) * scale(), cy = view.y + (event.clientY - box.top) * scale();
    zoom(Math.min(1.08, Math.max(1 / 1.08, Math.exp(event.deltaY * 0.0015))), cx, cy);
  };
  const onDown = (event: PointerEvent<SVGSVGElement>) => { event.preventDefault(); window.getSelection()?.removeAllRanges(); drag.current = { x: event.clientX, y: event.clientY, moved: false }; };
  const onMove = (event: PointerEvent<SVGSVGElement>) => {
    const start = drag.current;
    if (!start || event.buttons !== 1) return;
    const dx = event.clientX - start.x, dy = event.clientY - start.y;
    if (!start.moved && Math.hypot(dx, dy) < 4) return;
    if (!start.moved) svg.current?.setPointerCapture(event.pointerId);
    start.moved = true; start.x = event.clientX; start.y = event.clientY;
    const k = scale();
    setView((v) => ({ ...v, x: v.x - dx * k, y: v.y - dy * k }));
  };
  const onUp = () => { const was = drag.current?.moved; drag.current = null; return was; };
  const pick = (id: string) => { if (!drag.current?.moved) go(id); };
  return <>
    <div className="graph-toggles">
      {([['grants', 'Grants'], ['answers', 'Who answers for whom'], ['installed', 'Set up by the install']] as const).map(([key, label]) =>
        <button key={key} className={'chk' + (show[key] ? ' on' : '')} aria-pressed={show[key]} onClick={() => setShow({ ...show, [key]: !show[key] })}>{label}</button>)}
    </div>
    <div className="graph-split">
      <div className="graph-wrap">
        <div className="graph-zoom"><button className="btn" onClick={() => zoom(1 / 1.3)} aria-label="Zoom in">+</button><button className="btn" onClick={() => zoom(1.3)} aria-label="Zoom out">−</button><button className="btn" onClick={() => setView({ x: 0, y: 0, w: W, h: H })}>Fit</button></div>
        <svg ref={svg} viewBox={`${view.x} ${view.y} ${view.w} ${view.h}`} role="img" aria-label="Permission graph"
          onWheel={onWheel} onPointerDown={onDown} onPointerMove={onMove} onPointerUp={onUp} onPointerLeave={() => { drag.current = null; }}>
          {edges.map((edge, index) => {
            const a = at.get(edge.a), b = at.get(edge.b);
            if (!a || !b) return null;
            const state = hot ? (hot.has(edge.a) && hot.has(edge.b) && (edge.a === focus || edge.b === focus) ? ' hot' : ' dimmed') : '';
            return <line key={index} className={'ge ' + edge.kind + state} x1={a.x} y1={a.y} x2={b.x} y2={b.y}><title>{edge.label}</title></line>;
          })}
          {nodes.map((node) => <g key={node.id} className={'gn' + (hot && !hot.has(node.id) ? ' dimmed' : '')} transform={`translate(${node.x.toFixed(1)},${node.y.toFixed(1)})`}
            tabIndex={0} role="link" aria-label={node.label} onClick={() => pick(node.id)} onKeyDown={(event) => { if (event.key === 'Enter') go(node.id); }}>
            <Shape node={node} /><text x={13} y={4}>{node.label}</text>
          </g>)}
        </svg>
        <div className="legend">
          <span><i className="grant" />grant</span><span><i className="void" />does not stand</span><span><i className="answers" />answers to</span>
          <span>● person · ● agent · ■ app · ◆ resource</span>
        </div>
      </div>
      <div className="node-card">
        {!picked ? <><h2>Click anything</h2><p className="note">A person or agent lights up what it can reach. A resource lights up who can reach it. The same answers as the Access page, drawn.</p></>
          : picked.kind === 'resource' ? <><h2>{picked.label}</h2><p className="note">Highlighted: everyone who can reach it, per the permission service.</p>
            {reachedBy.map(([holder, actions]) => <div className="row" key={holder}><span>{nameOf(world, holder)}</span><span className="mono dim">{actions.join(', ')}</span></div>)}
            {!reachedBy.length ? <p className="dim">Nobody.</p> : null}
            <div className="actions"><a className="btn" href={'#/access/who/' + encodeURIComponent(picked.label)}>Open resource</a><a className="btn" href="#/graph">Clear</a></div></>
          : <><h2>{picked.label}</h2><p className="note">Highlighted: everything it can reach, per the permission service.</p>
            {reaches.map(({ resource, actions }) => <a className="row" key={resource} href={'#/access/who/' + encodeURIComponent(resource)}><span>{resource}</span><span className="mono dim">{actions.join(', ')}</span></a>)}
            {!reaches.length ? <p className="dim">Nothing.</p> : null}
            <div className="actions">{world.who.has(picked.id) ? <a className="btn" href={'#/file/' + encodeURIComponent(picked.id)}>Open file</a> : null}<a className="btn" href="#/graph">Clear</a></div></>}
      </div>
    </div>
  </>;
}

export function Graph() {
  const { id } = useParams();
  const load = useLoad(readGraph, 'identity-graph');
  return <div className="page"><div className="eyebrow">Access</div><h1>Graph</h1>
    <p className="sub">Every person, agent and resource, and the relations between them.</p>
    <Gate load={load} title="Graph" ok={({ world, reach, installed }) => <>
      <Drawing world={world} reach={reach} installed={installed} focus={id} />
      <p className="note">Answers are limited to the directory and resources you may see. These reads do not exercise a grant.</p>
    </>} />
  </div>;
}
