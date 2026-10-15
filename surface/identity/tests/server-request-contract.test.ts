import { expect, it } from 'vitest';
import contracts from './server-required-fields.json';
import { missingFields } from './server-request-contract';

it('rejects every missing required server field on each operation-bearing route', () => {
  for (const [route, fields] of Object.entries(contracts)) {
    const [method, path] = route.split(' ');
    const address = path.replace(/\{[^}]+\}/g, 'fixture');
    const complete = Object.fromEntries(fields.map((field) => [field, 'fixture']));
    expect(missingFields(method, address, complete), route).toEqual([]);
    for (const field of fields) {
      const incomplete = { ...complete };
      delete incomplete[field];
      expect(missingFields(method, address, incomplete), route + ' requires ' + field).toEqual([field]);
    }
  }
});
