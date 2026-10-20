/**
 * The acts a person holds on their own organisation (ACCESS-009 R4), read from their own grants: a live grant they
 * hold on `organisation:<their organisation>` carrying organisation.admit lets them add a person, one carrying
 * organisation.grant lets them give access directly. The service judges every act; this only decides what to offer.
 */
import type { MeView, OrganisationSummary } from '../../generated';
import type { Grant } from '../../generated/grants';

export const ORGANISATION_ADMIT = 'organisation.admit';
export const ORGANISATION_GRANT = 'organisation.grant';

export interface OrganisationActs {
  /** The organisation the person belongs to, or null when the service names none. */
  organisation: OrganisationSummary | null;
  /** Whether a standing grant of theirs on it carries organisation.admit. */
  admit: boolean;
  /** Whether a standing grant of theirs on it carries organisation.grant. */
  grant: boolean;
}

export function organisationActs(me: MeView, grants: Grant[]): OrganisationActs {
  const organisation = me.organisation ?? null;
  if (!organisation) return { organisation, admit: false, grant: false };
  const actions = new Set(grants
    .filter((g) => g.holder === me.person.id && g.standing.stands && g.resource.kind === 'organisation' && g.resource.id === organisation.id)
    .flatMap((g) => g.actions));
  return { organisation, admit: actions.has(ORGANISATION_ADMIT), grant: actions.has(ORGANISATION_GRANT) };
}
