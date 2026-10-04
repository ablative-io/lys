/** Names are not unique: a sentence that names an identity carries who each is in its title, and the Running canvas never shows a raw id. */
import { describe, expect, it } from 'vitest';
import { AnswerView } from '../src/features/grants/Answer';
import type { Answer } from '../src/features/grants/check';
import { namesTitle, voidOf } from '../src/features/grants/model';
import type { GrantWorld, Who } from '../src/features/grants/model';
import { graphFromRecords, withMessages } from '../src/features/runtime/session-graph';
import type { RuntimeSession } from '../src/features/runtime/RuntimeSessions';
import type { Grant } from '../src/generated/grants';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { ADA, BEA, COURIER, GRANTS, ME, MODEL, SCRIBE, SCRIBE_G } from './fixtures';

const hex = (n: number) => n.toString(16).padStart(32, '0');
const HIDDEN_AGENT = 'agent-' + hex(99);
const HIDDEN_PERSON = 'person-' + hex(98);

/** Two agents both named Scout, one of Ada's and one of Bea's. */
function world(): GrantWorld {
  const who = new Map<string, Who>([
    [ADA, { name: 'Ada', state: 'active', kind: 'person', responsible: null }],
    [BEA, { name: 'Bea', state: 'active', kind: 'person', responsible: null }],
    [SCRIBE, { name: 'Scout', state: 'active', kind: 'agent', responsible: ADA }],
    [COURIER, { name: 'Scout', state: 'active', kind: 'agent', responsible: BEA }],
  ]);
  const list = { grants: GRANTS, revision: 7 };
  return { me: ME, people: { people: [] } as unknown as GrantWorld['people'], list, model: MODEL, byId: new Map(GRANTS.map((g) => [g.id, g])), who } as GrantWorld;
}

describe('A sentence naming two of one name', () => {
  it('titles the service\'s sentence with who each identity it names is', () => {
    const w = world();
    const title = namesTitle(w, `${SCRIBE} and ${COURIER} were both refused`);
    expect(title).toBe(`Scout, agent of Ada, A/${SCRIBE.slice(6, 14)}; Scout, agent of Bea, A/${COURIER.slice(6, 14)}`);
    expect(namesTitle(w, 'nothing named here')).toBeUndefined();
  });

  it('carries the title on a refused answer, and on a void grant\'s reason', async () => {
    const w = world();
    const refusal = { ok: false, kind: 'NotHeld', why: `${SCRIBE} holds nothing here; ${COURIER} does`, revision: 7, t: '12:00' } as unknown as Answer;
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    await act(async () => { root.render(<AnswerView w={w} a={refusal} land={false} />); });
    const why = container.querySelector('.why');
    expect(why?.textContent).toBe('Scout holds nothing here; Scout does.');
    expect(why?.getAttribute('title')).toContain('Scout, agent of Ada');
    expect(why?.getAttribute('title')).toContain('Scout, agent of Bea');
    act(() => root.unmount());
    const suspended: Grant = { ...GRANTS.find((g) => g.id === SCRIBE_G) as Grant, standing: { stands: false, refusal: 'IdentityNotActive', grant: SCRIBE_G, reason: `${SCRIBE} is suspended` } };
    const v = voidOf(w, suspended);
    expect(v?.why).toBe('Scout is suspended');
    expect(v?.title).toBe(`Scout is suspended. Scout, agent of Ada, A/${SCRIBE.slice(6, 14)}`);
  });
});

describe('The Running canvas', () => {
  const session = (agent: string): RuntimeSession => ({ session: 's-' + agent, agent, machine: 'm-1', machine_name: 'Desk', runtime: null, shown: 'running', last_reported: '', first_report_at: 0, last_report_at: 0, what: '', stopped: null, reported_by: '' });

  it('says an agent outside the caller\'s view in words, never by its id', () => {
    const graph = graphFromRecords([session(HIDDEN_AGENT), session(SCRIBE)], [], world());
    const titles = graph.nodes.filter((node) => node.column === 'sessions').map((node) => node.title);
    expect(titles).toEqual(['an agent outside your view', 'Scout']);
  });

  it('says why a name is missing when the names could not be read', () => {
    const graph = graphFromRecords([session(HIDDEN_AGENT)], [], null);
    expect(graph.nodes[0].title).toBe('an agent whose name could not be read');
  });

  it('reads a resource by its kind in words, and its own id only when that is not an identity or operation id', () => {
    const w = world();
    const on = (kind: string, id: string): Grant => ({ ...GRANTS.find((g) => g.id === SCRIBE_G) as Grant, id: 'grant-' + kind + id, resource: { kind, id } });
    w.list = { grants: [on('project', 'ledger'), on('agent', HIDDEN_AGENT), on('service_account', 'op-' + hex(5)), on('agent', COURIER)], revision: 7 };
    const graph = graphFromRecords([session(SCRIBE)], [], w);
    const titles = graph.nodes.filter((node) => node.column === 'resources').map((node) => node.title);
    expect(titles).toEqual(['project ledger', 'agent', 'service account', 'agent Scout']);
    const read = [...graph.nodes.flatMap((node) => [node.title, node.detail]), ...graph.edges.map((edge) => edge.label)];
    expect(read.filter((words) => /(?:person|agent|op)-[0-9a-f]{8,}/.test(words))).toEqual([]);
  });

  it('says a message\'s sender and recipient outside the caller\'s view in words', () => {
    const edge = { message: 'post-1', stream: 'chat', source: HIDDEN_PERSON, recipients: [HIDDEN_AGENT], addressing: 'direct' as const, at: 1 };
    const seen = withMessages({ nodes: [], edges: [], names: {}, notices: [], unanswered: [] }, [edge]);
    expect(seen.nodes.map((node) => node.title)).toEqual(['someone outside your view', 'an agent outside your view']);
    const unread = withMessages({ nodes: [], edges: [], names: null, notices: [], unanswered: [] }, [edge]);
    expect(unread.nodes.map((node) => node.title)).toEqual(['someone whose name could not be read', 'an agent whose name could not be read']);
  });
});
