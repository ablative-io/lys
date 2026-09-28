import { kindOf } from '../../generated';
import type { PersonSummary } from '../../generated';

export function Pill({ x, extra = '' }: { x: PersonSummary; extra?: string }) {
  return (
    <a className={`pill ${kindOf(x.id) === 'person' ? 'human' : ''} ${extra}`} href={'#/file/' + x.id} style={{ textDecoration: 'none' }}>
      {x.display_name}
    </a>
  );
}
