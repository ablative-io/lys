import { conceptById } from './concepts';

/**
 * The help's state lives in the address, so a help page can
 * be linked and survives a reload. It sits in the query before the `#`, outside the hash router's own path and
 * query: opening help never changes the route a screen reads, so no screen reloads or loses what was typed in it,
 * and moving between screens keeps the help as it was.
 *
 * `help=1` shows the page help; `topic=index` opens the full help's index, `topic=<id>` one of its sheets.
 */
export const HELP_PARAM = 'help';
export const TOPIC_PARAM = 'topic';
export const INDEX_TOPIC = 'index';

export interface HelpUrlState {
  /** The page help (the numbered layer) is showing. */
  page: boolean;
  /** The full help is open: `index`, or a concept's id. Null when it is shut. */
  topic: string | null;
}

export const CLOSED_HELP: HelpUrlState = { page: false, topic: null };

/** The state an address carries. A topic naming no concept (an old link, a typo) opens the index instead. */
export function helpStateFromParams(params: URLSearchParams): HelpUrlState {
  const asked = params.get(TOPIC_PARAM);
  const topic = asked === null ? null : asked === INDEX_TOPIC || conceptById(asked) ? asked : INDEX_TOPIC;
  return { page: params.get(HELP_PARAM) === '1', topic };
}

/** `params` with the help's own members set to `state`; every other member is kept as it was. */
export function withHelpState(params: URLSearchParams, state: HelpUrlState): URLSearchParams {
  const next = new URLSearchParams(params);
  if (state.page) next.set(HELP_PARAM, '1');
  else next.delete(HELP_PARAM);
  if (state.topic === null) next.delete(TOPIC_PARAM);
  else next.set(TOPIC_PARAM, state.topic);
  return next;
}

/** The help state of the page's address now. */
export const readHelpUrl = (): HelpUrlState => helpStateFromParams(new URLSearchParams(window.location.search));

/**
 * Writes `state` into the address in place. The router's own history entry (its `state`) is kept, so Back and
 * Forward still move between screens; a change of help is not a step of its own.
 */
export function writeHelpUrl(state: HelpUrlState): void {
  const query = withHelpState(new URLSearchParams(window.location.search), state).toString();
  const url = window.location.pathname + (query ? '?' + query : '') + window.location.hash;
  window.history.replaceState(window.history.state, '', url);
}
