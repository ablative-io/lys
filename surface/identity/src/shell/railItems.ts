// The rail in CONSENSUS.md's order (18 October 2026): what a person uses most sits first, the service's own pages
// after, and the person's own things at the foot. Jobs and the access map are reached from Access, by key and by Search.

import { COMPUTERS } from '../features/network/labels';

export type RailItem =
  | { t: 'sep' }
  | { t: 'grow' }
  | {
    t: 'a' | 'button'; title: string; label: string; svg: string; href?: string; nav?: string; dock?: 'help'; id?: string;
    /** A key drawn beside the word. Only Help and Search carry one: every other key is in the title, on hover, so the rail reads as words. */
    kbd?: string;
    cnt?: string;
  };

export const RAIL: RailItem[] = [
  {"t": "a", "title": "Dashboard (g d)", "label": "Dashboard", "svg": "<path d=\"M4 11.5 12 5l8 6.5\"/><path d=\"M6 10v9.5h12V10\"/><path d=\"M10 19.5v-5h4v5\"/>", "href": "#/", "nav": "dashboard"},
  {"t": "a", "title": "Waiting on you (g q)", "label": "Waiting on you", "svg": "<path d=\"M4 13.5 6.5 5h11l2.5 8.5V19H4z\"/><path d=\"M4 13.5h4.5l1 2h5l1-2H20\"/>", "href": "#/?section=waiting-on-you", "nav": "waiting"},
  {"t": "sep"},
  {"t": "a", "title": "People and agents (g p)", "label": "People and agents", "svg": "<circle cx=\"9\" cy=\"8\" r=\"3.2\"/><path d=\"M3.5 19c.8-3.2 3-5 5.5-5s4.7 1.8 5.5 5\"/><circle cx=\"17\" cy=\"9\" r=\"2.4\"/><path d=\"M15.5 14.2c2.3.2 4 1.8 4.6 4.8\"/>", "href": "#/people", "nav": "people"},
  {"t": "a", "title": "Running agents (g c)", "label": "Running agents", "svg": "<circle cx=\"12\" cy=\"12\" r=\"8.5\"/><path d=\"M10 8.8v6.4l5.2-3.2z\"/>", "href": "#/canvas", "nav": "canvas"},
  {"t": "sep"},
  {"t": "a", "title": "Access (g a)", "label": "Access", "svg": "<circle cx=\"8\" cy=\"15\" r=\"3.5\"/><path d=\"M10.5 12.5 19 4M16 7l2.5 2.5M14 9l2 2\"/>", "href": "#/access", "nav": "access"},
  {"t": "a", "title": `${COMPUTERS} (g t)`, "label": COMPUTERS, "svg": "<rect x=\"5\" y=\"5.5\" width=\"14\" height=\"9.5\" rx=\"1.2\"/><path d=\"M3 18.5h18\"/>", "href": "#/network", "nav": "network"},
  {"t": "a", "title": "Apps (g n)", "label": "Apps", "svg": "<rect x=\"4\" y=\"4\" width=\"6.5\" height=\"6.5\" rx=\"1.5\"/><rect x=\"13.5\" y=\"4\" width=\"6.5\" height=\"6.5\" rx=\"1.5\"/><rect x=\"4\" y=\"13.5\" width=\"6.5\" height=\"6.5\" rx=\"1.5\"/><path d=\"M16.75 13.5v6.5M13.5 16.75H20\"/>", "href": "#/apps", "nav": "apps"},
  {"t": "a", "title": "Secrets (g v)", "label": "Secrets", "svg": "<rect x=\"4\" y=\"10\" width=\"16\" height=\"10\" rx=\"2\"/><path d=\"M8 10V7a4 4 0 0 1 8 0v3\"/><circle cx=\"12\" cy=\"15\" r=\"1.3\"/>", "href": "#/secrets", "nav": "secrets"},
  {"t": "grow"},
  {"t": "a", "title": "Settings (g s)", "label": "Settings", "svg": "<path d=\"M4 7h10M18 7h2M4 17h4M12 17h8\"/><circle cx=\"16\" cy=\"7\" r=\"2\"/><circle cx=\"10\" cy=\"17\" r=\"2\"/>", "href": "#/settings/account", "nav": "settings"},
  {"t": "a", "title": "You (g u)", "label": "You", "svg": "<circle cx=\"12\" cy=\"8\" r=\"3.5\"/><path d=\"M5 20c1-4 3.8-6 7-6s6 2 7 6\"/>", "href": "#/me", "nav": "me"},
  {"t": "button", "title": "Help (?)", "label": "Help", "svg": "<circle cx=\"12\" cy=\"12\" r=\"8.5\"/><path d=\"M9.6 9.5a2.5 2.5 0 1 1 3.4 2.3c-.6.3-1 .8-1 1.5v.4M12 16.8v.01\"/>", "dock": "help", "kbd": "?"},
  {"t": "sep"},
  {"t": "button", "title": "Search (⌘K)", "label": "Search", "svg": "<circle cx=\"11\" cy=\"11\" r=\"6\"/><path d=\"m15.5 15.5 4 4\"/>", "id": "palBtn", "kbd": "⌘K"},
  {"t": "button", "title": "Menu labels ([)", "label": "Menu labels", "svg": "<rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/><path d=\"M9 4v16\"/>", "id": "railBtn"},
];
