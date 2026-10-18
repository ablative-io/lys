import { afterEach, expect, it } from 'vitest';
import { $$, mount, settle, unmountAll } from './harness';
import { ADA, COURIER, DIRECTORY, SCRIBE, SERVICE, ok } from './fixtures';

afterEach(() => unmountAll());

// The service serves each agent's reports_to edge: Courier reports to the agent Scribe, Scribe to its person Ada.
const reporting = { ...DIRECTORY, people: DIRECTORY.people.map((person) => person.id !== ADA ? person : { ...person, agents: person.agents.map((agent) =>
  agent.id === COURIER ? { ...agent, reports_to: { id: SCRIBE, kind: 'agent', display_name: 'Scribe' } } : agent) }) };

it('says the agent an agent reports to, with the person who answers for it, and a person-reporting agent by its person', async () => {
  await mount('#/people/view/agents', { ...SERVICE, '/directory/people': ok(reporting) });
  const answersTo = (name: string) => $$('tr').find((row) => row.querySelector('td')?.textContent === name)?.querySelectorAll('td')[3]?.textContent;
  expect(answersTo('Courier')).toBe('Scribe (agent), for Ada (test person)');
  expect(answersTo('Scribe')).toBe('Ada (test person)');
  expect(document.body.textContent).toContain('Every agent has a person who answers for it.');
  expect(document.body.textContent).not.toContain('Every agent answers to a person.');
});

it('says the same on the agent’s own page: the agent it reports to, then its person', async () => {
  // Here Scribe, whose file the fixtures serve, reports to the agent Courier.
  const scribeReports = { ...DIRECTORY, people: DIRECTORY.people.map((person) => person.id !== ADA ? person : { ...person, agents: person.agents.map((agent) =>
    agent.id === SCRIBE ? { ...agent, reports_to: { id: COURIER, kind: 'agent', display_name: 'Courier' } } : agent) }) };
  await mount('#/file/' + SCRIBE, { ...SERVICE, '/directory/people': ok(scribeReports) });
  await settle();
  const fact = [...document.querySelectorAll('.agent-facts dt')].find((term) => term.textContent === 'Answers to')?.nextElementSibling;
  expect(fact?.textContent).toContain('Courier (agent), for ');
  expect(fact?.querySelector('a')?.getAttribute('href')).toBe('#/file/' + COURIER);
});
