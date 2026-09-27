import { useLocation } from 'react-router';

interface Screen {
  eyebrow: string;
  title: string;
  sub: string;
  why: string;
}

/** Each screen the rail reaches whose server does not exist yet, with the mock-up's own heading. */
export const SCREENS: Record<string, Screen> = {
  roles: { eyebrow: 'Directory', title: 'Roles', sub: 'A role is a job and a starting point: what the holder is for, the profile an agent starts from, and the grants it usually needs.', why: 'Roles wait on their ADR (conformance 4.1 to 4.7).' },
  resources: { eyebrow: 'Access', title: 'Resources', sub: "Everything access can be granted on. This service's own types need nothing else installed; other products add theirs when they connect.", why: 'Resources come with the grant model in DIRECTORY-006 R1 to R5.' },
  access: { eyebrow: 'Access', title: 'Access', sub: 'Ask it any way round. Every answer traces to a person, or says why not.', why: 'The explanation seam is DIRECTORY-006 R5 (conformance 8.1, 8.2).' },
  graph: { eyebrow: 'Access', title: 'Graph', sub: 'Every person, agent and resource, and the relations between them.', why: 'The graph draws the answers Access gives (conformance 8.3), which arrive with DIRECTORY-006 R5.' },
  requests: { eyebrow: 'Access', title: 'Requests', sub: 'Asked by people and agents alike, through the screen or through their tools.', why: 'Requests wait for grants (DIRECTORY-006).' },
  reviews: { eyebrow: 'Access', title: 'Reviews', sub: 'Each person confirms, now and then, what the agents they answer for still need.', why: 'Reviews wait for grants (DIRECTORY-006).' },
  secrets: { eyebrow: 'Runtime', title: 'Secrets', sub: 'The store holds the real values. Everything anyone uses is made from it, scoped, and can be taken back.', why: 'The secrets broker is SECRETS-002 (conformance 7.1 to 7.8).' },
  connections: { eyebrow: 'Runtime', title: 'Connections', sub: 'Both directions: products that ask this service before they act, and systems agents reach through it.', why: 'No connection registry exists yet.' },
  network: { eyebrow: 'Runtime', title: 'Network', sub: 'The machines agents can run on, what each may reach, and which agents may run where.', why: 'The network view is proposed (conformance 8.5).' },
  sessions: { eyebrow: 'Runtime', title: 'Sessions', sub: 'Who is working where, and for whom, as each runtime reports it. The live stream of what they are doing belongs to the runtime.', why: 'Sessions come with launch records and runtime reports (conformance 5.4 to 5.6).' },
  model: { eyebrow: 'Model', title: 'Model', sub: 'What can be granted, on what, and what each grant allows. Versioned; every change is tried first.', why: 'The model comes with the grant contract in DIRECTORY-006 R1.' },
};

export function NotYet() {
  const { pathname } = useLocation();
  const view = pathname.split('/')[1] === 'vault' ? 'secrets' : pathname.split('/')[1];
  const screen = SCREENS[view];
  if (!screen) return null;
  return (
    <div className="page">
      <div className="eyebrow">{screen.eyebrow}</div>
      <h1>{screen.title}</h1>
      <p className="sub">{screen.sub}</p>
      <div className="empty-note">
        <span className="open-q">not built yet</span> {screen.why} Nothing is shown here until its server answers.
      </div>
    </div>
  );
}
