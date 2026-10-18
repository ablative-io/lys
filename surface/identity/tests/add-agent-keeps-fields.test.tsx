/** A computer whose installed programs cannot be read never wipes what the person already typed or chose. */
import { describe, expect, it } from 'vitest';
import { $, settle, type } from './harness';
import { refused, ok } from './fixtures';
import { program, service, open, existingComputer, chooseComputer } from './add-and-run-fixture';

describe('Add an agent keeps its fields', () => {
  it('keeps the name and the computer chosen when that computer’s programs cannot be read', async () => {
    const good = existingComputer('e', 'Front desk');
    const bad = existingComputer('b', 'Back desk');
    const server = service(undefined, 'running', [good, bad]);
    server.routes['/harnesses'] = ok({ programs: [program] });
    server.routes['/harnesses?machine=' + good.id] = ok({ machine: good.id, programs: [program] });
    server.routes['/harnesses?machine=' + bad.id] = refused(502, 'runner_unreachable', 'the runner did not answer');
    await open(server.routes);
    await type($('[name="display_name"]'), 'Clover');
    await chooseComputer(bad.id);
    await settle();
    expect(($('[name="display_name"]') as HTMLInputElement).value).toBe('Clover');
    expect(($('[name="computer"]') as HTMLSelectElement).value).toBe(bad.id);
    expect(document.body.textContent).toContain('Lys could not read the programs installed on');
    await chooseComputer(good.id);
    await settle();
    expect(($('[name="display_name"]') as HTMLInputElement).value).toBe('Clover');
    expect(($('[name="computer"]') as HTMLSelectElement).value).toBe(good.id);
  });
});
