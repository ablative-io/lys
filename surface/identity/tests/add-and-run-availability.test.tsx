import { beforeEach, describe, expect, it } from 'vitest';
import type { Machine } from '../src/features/network/contract';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Answer, Route } from './fixtures';
import { choose, click, mount, settle, type } from './harness';

const program = {
  name: 'Care program', line: 'Care program',
  models: [{ id: 'care', label: 'Care model' }],
  modes: [{ id: 'workspace-write', meaning: 'Works in its own folder; no internet.' }],
  instructions_modes: ['keep'],
  builds: [{ name: 'Installed', program: '/opt/bin/care', package: 'care', from: 'runner' }],
  description: {
    models: { minimum: 1, maximum: 1, further_encoding: { kind: 'array' } },
    permissions: { modes: ['workspace-write'], rule_forms: [] },
    mcp: { transports: ['stdio'], working_directory: false, handle_variables: false, channel_policies: ['off'] },
    rendering_contract: 'test',
  },
};

function computer(digit: string, name: string): Machine {
  return {
    id: 'op-' + digit.repeat(32), name, kind: 'Computer', runtime: 'lys-runner', slots: 0,
    may_run: [{ id: SCRIBE, display_name: 'Scribe', state: 'active' }], may_run_roles: [], may_reach: [],
    named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: 1,
  };
}

const runnerPath = (machine: Machine) => '/network/machines/' + machine.id + '/runner';
const confirmed = (machine: Machine) => ok({ machine: machine.id, runner: { kind: 'lys' } });

function routes(computers: Machine[], extra: Record<string, Route> = {}): Record<string, Route> {
  const contract = SERVICE['/surface-contract'];
  if (typeof contract === 'function') throw new Error('The surface contract fixture must be an answer');
  const schema = contract.body as { paths: Record<string, unknown> };
  const served: Record<string, Route> = {
    ...SERVICE,
    '/surface-contract': ok({ paths: {
      ...schema.paths,
      '/agents': { post: { requestBody: { content: { 'application/json': { schema: { properties: { operation: {}, display_name: {}, answers_to: {} } } } } } } },
    } }),
    '/network': ok({ machines: computers, reports_served: true }),
    '/harnesses': ok({ programs: [program] }),
    '/skills': ok({ skills: [] }),
  };
  for (const machine of computers) {
    served[runnerPath(machine)] = confirmed(machine);
    served['POST /network/machines/' + machine.id + '/folders'] = ok({ machine: machine.id, under: '/srv', folders: [] });
  }
  return { ...served, ...extra };
}

function pendingAnswer() {
  let answer: ((value: Answer) => void) | null = null;
  const pending = new Promise<Answer>((resolve) => { answer = resolve; });
  return {
    route: (() => pending) as unknown as Route,
    answer(value: Answer) {
      if (!answer) throw new Error('The deferred fixture has no answer callback');
      answer(value);
    },
  };
}

function form(container: HTMLElement): HTMLFormElement {
  const found = container.querySelector<HTMLFormElement>('form[aria-label="Add an agent"]');
  if (!found) throw new Error('The add-agent form is missing');
  return found;
}

function submit(container: HTMLElement): HTMLButtonElement {
  const found = form(container).querySelector<HTMLButtonElement>('button[type="submit"]');
  if (!found) throw new Error('The add-agent submit button is missing');
  return found;
}

function offered(container: HTMLElement): string[] {
  return [...form(container).querySelectorAll<HTMLOptionElement>('select[name="computer"] option')].map((option) => option.value);
}

async function ready(container: HTMLElement, name = 'Clover') {
  await type(form(container).querySelector('input[name="display_name"]'), name);
  await click(form(container).querySelector('button[aria-label="Choose a folder"]'));
  await type(form(container).querySelector('input[name="folder-path"]'), '/srv');
  await click(form(container).querySelector('button[aria-label="Open the folder typed"]'));
  expect(form(container).querySelector('.works-in code')?.textContent).toBe('/srv');
}

beforeEach(() => sessionStorage.clear());

describe('Add-agent runner availability', () => {
  it('offers the confirmed computer when the first listed computer refuses, with the refusal reason verbatim', async () => {
    const laptop = computer('a', 'Away laptop');
    const local = computer('b', 'Own computer');
    const reason = 'The registered runner is dialled, but nobody holds its bridge.';
    const { container, posted } = await mount('#/agents/new', routes([laptop, local], {
      [runnerPath(laptop)]: refused(503, 'RunnerUnreachable', reason),
    }));

    expect(form(container).textContent).toContain(laptop.name + ': not available: ' + reason);
    expect(offered(container)).not.toContain(laptop.id);
    expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.selectedOptions[0]?.textContent).toBe(local.name);
    expect(form(container).textContent).not.toContain('Ask the administrator to check the computer');
    await ready(container);
    expect(submit(container).textContent).toBe('Add and run');
    expect(submit(container).disabled).toBe(false);
    expect(posted.filter((entry) => entry.path === '/agents')).toEqual([]);
  });

  it('keeps plain Add enabled when no computer is confirmed, blocks only the run, and names every computer and its distinct answer', async () => {
    const laptop = computer('a', 'Away laptop');
    const terminal = computer('b', 'Ward terminal');
    const laptopReason = 'The laptop runner has no bridge holder.';
    const terminalReason = 'The terminal runner is not dialled in.';
    const { container, posted, requests } = await mount('#/agents/new', routes([laptop, terminal], {
      [runnerPath(laptop)]: refused(503, 'RunnerUnreachable', laptopReason),
      [runnerPath(terminal)]: refused(503, 'RunnerNotDialled', terminalReason),
    }));

    await type(form(container).querySelector('input[name="display_name"]'), 'Clover');
    expect(requests).toContain(runnerPath(laptop));
    expect(requests).toContain(runnerPath(terminal));
    expect(form(container).textContent).toContain(laptop.name + ': not available: ' + laptopReason);
    expect(form(container).textContent).toContain(terminal.name + ': not available: ' + terminalReason);
    expect(form(container).querySelector('[role="alert"]')).not.toBeNull();
    expect(offered(container)).not.toContain(laptop.id);
    expect(offered(container)).not.toContain(terminal.id);
    // Registering now and starting later never waits on a runner: only Add and run is withheld.
    expect(submit(container).textContent).toBe('Add');
    expect(submit(container).getAttribute('aria-label')).toBe('Add agent');
    expect(submit(container).disabled).toBe(false);
    expect(form(container).textContent).not.toContain('Add and run');
    expect(posted.filter((entry) => entry.path === '/agents')).toEqual([]);
  });

  it('asks every runner in parallel and offers each confirmed answer without waiting for the unanswered machines', async () => {
    const local = computer('a', 'Own computer');
    const laptop = computer('b', 'Away laptop');
    const terminal = computer('c', 'Ward terminal');
    const localReply = pendingAnswer();
    const laptopReply = pendingAnswer();
    const terminalReply = pendingAnswer();
    const reason = 'The laptop bridge has no holder.';
    try {
      const { container, requests } = await mount('#/agents/new', routes([local, laptop, terminal], {
        [runnerPath(local)]: localReply.route,
        [runnerPath(laptop)]: laptopReply.route,
        [runnerPath(terminal)]: terminalReply.route,
      }));

      for (const machine of [local, laptop, terminal]) {
        expect(requests).toContain(runnerPath(machine));
        expect(form(container).textContent).toContain(machine.name + ': asking its runner…');
      }
      expect(submit(container).disabled).toBe(true);

      localReply.answer(confirmed(local));
      await settle();
      // The first runner to answer is offered, never chosen for the person: other runners have not answered yet.
      expect(offered(container)).toEqual(['', local.id]);
      expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.value).toBe('');
      expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.value).toBe('');
      expect(form(container).textContent).not.toContain(local.name + ': asking its runner…');
      expect(form(container).textContent).toContain(laptop.name + ': asking its runner…');
      expect(form(container).textContent).toContain(terminal.name + ': asking its runner…');
      await type(form(container).querySelector('input[name="display_name"]'), 'Clover from the first answer');
      expect(submit(container).disabled).toBe(true);
      expect(form(container).textContent).toContain('Choose the computer this agent runs on.');
      await choose(form(container).querySelector('select[name="computer"]'), local.id);
      await ready(container, 'Clover from the first answer');
      expect(submit(container).disabled).toBe(false);

      laptopReply.answer(refused(503, 'RunnerUnreachable', reason));
      await settle();
      expect(form(container).textContent).toContain(laptop.name + ': not available: ' + reason);
      expect(form(container).textContent).not.toContain(laptop.name + ': asking its runner…');
      expect(form(container).textContent).toContain(terminal.name + ': asking its runner…');
      expect(submit(container).disabled).toBe(false);

      terminalReply.answer(confirmed(terminal));
      await settle();
      expect(offered(container)).toContain(local.id);
      expect(offered(container)).toContain(terminal.id);
      expect(offered(container)).not.toContain(laptop.id);
      expect(form(container).textContent).not.toContain('asking its runner…');
      expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.value).toBe(local.id);
      expect(form(container).querySelector<HTMLInputElement>('input[name="display_name"]')?.value).toBe('Clover from the first answer');
      expect(form(container).querySelector('.works-in code')?.textContent).toBe('/srv');
      expect(submit(container).disabled).toBe(false);
    } finally {
      localReply.answer(confirmed(local));
      laptopReply.answer(refused(503, 'RunnerUnreachable', reason));
      terminalReply.answer(confirmed(terminal));
      await settle();
    }
  });

  it('keeps an early refusal separate from an unanswered runner and retains the name when that runner confirms', async () => {
    const laptop = computer('a', 'Away laptop');
    const local = computer('b', 'Own computer');
    const laptopReply = pendingAnswer();
    const localReply = pendingAnswer();
    const reason = 'The laptop is registered without a reachable bridge.';
    try {
      const { container } = await mount('#/agents/new', routes([laptop, local], {
        [runnerPath(laptop)]: laptopReply.route,
        [runnerPath(local)]: localReply.route,
      }));
      await type(form(container).querySelector('input[name="display_name"]'), 'Clover named before confirmation');

      laptopReply.answer(refused(503, 'RunnerUnreachable', reason));
      await settle();
      expect(form(container).textContent).toContain(laptop.name + ': not available: ' + reason);
      expect(form(container).textContent).toContain(local.name + ': asking its runner…');
      expect(form(container).textContent).not.toContain('No in-use computer has a confirmed local Lys runner.');
      expect(submit(container).disabled).toBe(true);

      localReply.answer(confirmed(local));
      await settle();
      expect(form(container).querySelector<HTMLInputElement>('input[name="display_name"]')?.value).toBe('Clover named before confirmation');
      expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.selectedOptions[0]?.textContent).toBe(local.name);
      expect(form(container).textContent).toContain(laptop.name + ': not available: ' + reason);
      await click(form(container).querySelector('button[aria-label="Choose a folder"]'));
      await type(form(container).querySelector('input[name="folder-path"]'), '/srv');
      await click(form(container).querySelector('button[aria-label="Open the folder typed"]'));
      expect(submit(container).disabled).toBe(false);
    } finally {
      laptopReply.answer(refused(503, 'RunnerUnreachable', reason));
      localReply.answer(confirmed(local));
      await settle();
    }
  });

  it('offers only confirmed computers in the folder picker while another runner remains unanswered', async () => {
    const frontDesk = computer('a', 'Front desk');
    const local = computer('b', 'Own computer');
    const laptop = computer('c', 'Away laptop');
    const laptopReply = pendingAnswer();
    try {
      const { container } = await mount('#/agents/new', routes([frontDesk, local, laptop], {
        [runnerPath(laptop)]: laptopReply.route,
      }));
      await choose(form(container).querySelector('select[name="computer"]'), '');
      expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.value).toBe('');
      await click(form(container).querySelector('button[aria-label="Choose a folder"]'));

      const picker = form(container).querySelector<HTMLSelectElement>('select[name="folder-computer"]');
      if (!picker) throw new Error('The folder computer picker is missing');
      expect([...picker.options].map((option) => option.value)).toEqual(['', frontDesk.id, local.id]);
      expect(form(container).textContent).toContain(laptop.name + ': asking its runner…');
    } finally {
      laptopReply.answer(confirmed(laptop));
      await settle();
    }
  });

  it('shows a confirmed computer before programme choices answer without waiting for its unanswered peer', async () => {
    const local = computer('a', 'Own computer');
    const laptop = computer('b', 'Away laptop');
    const programmesReply = pendingAnswer();
    const laptopReply = pendingAnswer();
    const reason = 'The laptop runner has no bridge holder.';
    try {
      const { container, requests } = await mount('#/agents/new', routes([local, laptop], {
        '/harnesses': programmesReply.route,
        [runnerPath(laptop)]: laptopReply.route,
      }));

      expect(requests).toContain('/harnesses');
      expect(requests).toContain(runnerPath(local));
      expect(requests).toContain(runnerPath(laptop));
      expect(offered(container)).toEqual(['', local.id]);
      expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.value).toBe('');
      expect(form(container).textContent).toContain(laptop.name + ': asking its runner…');
      expect(form(container).querySelector('select[name="program"]')).toBeNull();
      expect(submit(container).disabled).toBe(true);
      await type(form(container).querySelector('input[name="display_name"]'), 'Clover waiting for programmes');

      programmesReply.answer(ok({ programs: [program] }));
      await settle();
      expect(form(container).querySelector<HTMLInputElement>('input[name="display_name"]')?.value).toBe('Clover waiting for programmes');
      expect(form(container).textContent).toContain(laptop.name + ': asking its runner…');
      await choose(form(container).querySelector('select[name="computer"]'), local.id);
      await click(form(container).querySelector('button[aria-label="Choose a folder"]'));
      await type(form(container).querySelector('input[name="folder-path"]'), '/srv');
      await click(form(container).querySelector('button[aria-label="Open the folder typed"]'));
      expect(submit(container).disabled).toBe(false);

      laptopReply.answer(refused(503, 'RunnerUnreachable', reason));
      await settle();
      expect(form(container).textContent).toContain(laptop.name + ': not available: ' + reason);
      expect(submit(container).disabled).toBe(false);
    } finally {
      programmesReply.answer(ok({ programs: [program] }));
      laptopReply.answer(refused(503, 'RunnerUnreachable', reason));
      await settle();
    }
  });

  it('runs on the computer of a copy picked from its group, and the computer choice follows it', async () => {
    const frontDesk = computer('a', 'Front desk');
    const local = computer('b', 'Own computer');
    const { container } = await mount('#/agents/new', routes([frontDesk, local]));
    expect(offered(container)).toEqual(['', frontDesk.id, local.id]);
    expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.value).toBe('');
    const copies = form(container).querySelector<HTMLSelectElement>('select[name="build"]');
    const group = [...(copies?.querySelectorAll('optgroup') ?? [])].find((entry) => entry.label === local.name);
    const option = group?.querySelector('option');
    if (!option) throw new Error('The Own computer copy is missing');
    await choose(copies, option.value);
    expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.value).toBe(local.id);
    await ready(container, 'Clover on the picked computer');
    expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.value).toBe(local.id);
    expect(submit(container).disabled).toBe(false);
  });

  it('keeps the name, chosen folder and explicit computer choice when a later runner refuses', async () => {
    const frontDesk = computer('a', 'Front desk');
    const local = computer('b', 'Own computer');
    const laptop = computer('c', 'Away laptop');
    const laptopReply = pendingAnswer();
    const reason = 'Nobody holds the laptop runner bridge.';
    try {
      const { container, posted } = await mount('#/agents/new', routes([frontDesk, local, laptop], {
        [runnerPath(laptop)]: laptopReply.route,
      }));
      await choose(form(container).querySelector('select[name="computer"]'), local.id);
      await ready(container, 'Clover with chosen settings');
      expect(form(container).textContent).toContain(laptop.name + ': asking its runner…');
      expect(submit(container).disabled).toBe(false);

      laptopReply.answer(refused(503, 'RunnerUnreachable', reason));
      await settle();

      expect(form(container).textContent).toContain(laptop.name + ': not available: ' + reason);
      expect(form(container).querySelector<HTMLInputElement>('input[name="display_name"]')?.value).toBe('Clover with chosen settings');
      expect(form(container).querySelector<HTMLSelectElement>('select[name="computer"]')?.value).toBe(local.id);
      expect(form(container).querySelector('.works-in code')?.textContent).toBe('/srv');
      expect(offered(container)).toEqual(['', frontDesk.id, local.id]);
      expect(submit(container).disabled).toBe(false);
      expect(posted.filter((entry) => entry.path === '/agents')).toEqual([]);
    } finally {
      laptopReply.answer(refused(503, 'RunnerUnreachable', reason));
      await settle();
    }
  });
});
