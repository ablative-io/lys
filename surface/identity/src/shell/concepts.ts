// The help concepts, word for word as the mock-up gives them.

export interface Concept {
  id: string;
  t: string;
  s: string;
  sel: string;
  body: string;
}

export const CONCEPTS: Concept[] = [
  {"id": "grant", "t": "Grant", "s": "Who holds which relation on what, and the person it traces back to.", "sel": ".chain", "body": "Every grant is five facts: who holds it, the relation, the object, who it derives from, and whether it may be passed on. Withdrawing a grant withdraws everything derived from it."},
  {"id": "state", "t": "State", "s": "Registered, active, suspended or retired. Authority only.", "sel": ".state, .state-of", "body": "State says what an identity is allowed to be doing, not whether anything is running. Sessions are counted separately, as the runtimes report them."},
  {"id": "kind", "t": "Person or agent", "s": "Both are identities. Every agent answers to a person.", "sel": ".kind", "body": "People sign in. Agents are registered by a person who then answers for them. Anything a person can do here, an agent can do through its tools, if it holds the grant."},
  {"id": "check", "t": "Can this be done?", "s": "Ask any way round; every answer shows its path.", "sel": ".check", "body": "Yes answers show the chain to a person. No answers say which kind of refusal it is: no grant, identity not active, authority withdrawn upstream, or a type the model does not have."},
  {"id": "svc", "t": "Where a type comes from", "s": "Built in, or added by a connected product.", "sel": ".svc", "body": "This service has its own types and works with nothing else installed. Other products, ours or not, add theirs when a person approves them."},
  {"id": "sealed", "t": "Sealed value", "s": "Never shown, to anyone.", "sel": ".sealed", "body": "Secret values go into the store once. Agents use virtual credentials made from them; people use them through the same."},
  {"id": "stat", "t": "At a glance", "s": "Counts for this screen.", "sel": ".stat-strip", "body": "Each number opens the list behind it."},
  {"id": "graph", "t": "Graph", "s": "The same answers as Access, drawn.", "sel": "#gsvg", "body": "Click a person or agent to light what it reaches; click a resource to light who reaches it."},
  {"id": "scope", "t": "Scope", "s": "Organisation, team, or yours alone.", "sel": "[data-scope]", "body": "A personal secret is yours. An agent uses it only through a virtual credential you issue, acting for you."},
  {"id": "tabs", "t": "Sections", "s": "Each section has its own address.", "sel": ".tabs", "body": "Keys 1 to 7 switch sections of a file. Every section can be linked to."},
  {"id": "rail", "t": "The rail", "s": "Every screen, one key away.", "sel": "#rail", "body": "g then a letter goes to a screen. [ shows or hides labels. The rail and dock can sit on either side."},
];
