// The rail, item for item as the mock-up draws it. Icon paths are its own.

export type RailItem =
  | { t: 'sep' }
  | { t: 'grow' }
  | { t: 'a' | 'button'; title: string; label: string; svg: string; href?: string; nav?: string; dock?: 'help' | 'assistant'; id?: string; kbd?: string; cnt?: string };

export const RAIL: RailItem[] = [
  {"t": "a", "title": "You (g u)", "label": "You", "svg": "<circle cx=\"12\" cy=\"8\" r=\"3.5\"/><path d=\"M5 20c1-4 3.8-6 7-6s6 2 7 6\"/>", "href": "#/me", "nav": "me", "kbd": "g u"},
  {"t": "a", "title": "People and agents (g p)", "label": "People and agents", "svg": "<circle cx=\"9\" cy=\"8\" r=\"3.2\"/><path d=\"M3.5 19c.8-3.2 3-5 5.5-5s4.7 1.8 5.5 5\"/><circle cx=\"17\" cy=\"9\" r=\"2.4\"/><path d=\"M15.5 14.2c2.3.2 4 1.8 4.6 4.8\"/>", "href": "#/people", "nav": "people", "kbd": "g p"},
  {"t": "a", "title": "Running (g c)", "label": "Running", "svg": "<rect x=\"3\" y=\"4\" width=\"7\" height=\"6\" rx=\"1.2\"/><rect x=\"14\" y=\"4\" width=\"7\" height=\"6\" rx=\"1.2\"/><rect x=\"8.5\" y=\"14\" width=\"7\" height=\"6\" rx=\"1.2\"/><path d=\"M10 7h4M6.5 10l3.5 4M17.5 10 14 14\"/>", "href": "#/canvas", "nav": "canvas", "kbd": "g c"},
  {"t": "a", "title": "Roles (g o)", "label": "Roles", "svg": "<rect x=\"4\" y=\"7\" width=\"16\" height=\"12\" rx=\"2\"/><path d=\"M9 7V5.5A1.5 1.5 0 0 1 10.5 4h3A1.5 1.5 0 0 1 15 5.5V7\"/><path d=\"M4 12h16\"/>", "href": "#/roles", "nav": "roles", "kbd": "g o"},
  {"t": "sep"},
  {"t": "a", "title": "Access (g a)", "label": "Access", "svg": "<circle cx=\"8\" cy=\"15\" r=\"3.5\"/><path d=\"M10.5 12.5 19 4M16 7l2.5 2.5M14 9l2 2\"/>", "href": "#/access", "nav": "access", "kbd": "g a"},
  {"t": "sep"},
  {"t": "a", "title": "Secrets (g v)", "label": "Secrets", "svg": "<rect x=\"4\" y=\"10\" width=\"16\" height=\"10\" rx=\"2\"/><path d=\"M8 10V7a4 4 0 0 1 8 0v3\"/><circle cx=\"12\" cy=\"15\" r=\"1.3\"/>", "href": "#/secrets", "nav": "secrets", "kbd": "g v"},
  {"t": "a", "title": "Computers (g t)", "label": "Computers", "svg": "<rect x=\"3\" y=\"4\" width=\"7\" height=\"5\" rx=\"1\"/><rect x=\"14\" y=\"4\" width=\"7\" height=\"5\" rx=\"1\"/><rect x=\"8.5\" y=\"15\" width=\"7\" height=\"5\" rx=\"1\"/><path d=\"M6.5 9v3h11V9M12 12v3\"/>", "href": "#/network", "nav": "network", "kbd": "g t"},
  {"t": "grow"},
  {"t": "button", "title": "Help (?)", "label": "Help", "svg": "<circle cx=\"12\" cy=\"12\" r=\"8.5\"/><path d=\"M9.6 9.5a2.5 2.5 0 1 1 3.4 2.3c-.6.3-1 .8-1 1.5v.4M12 16.8v.01\"/>", "dock": "help", "kbd": "?"},
  {"t": "a", "title": "Configuration (g s)", "label": "Configuration", "svg": "<path d=\"M4 7h10M18 7h2M4 17h4M12 17h8\"/><circle cx=\"16\" cy=\"7\" r=\"2\"/><circle cx=\"10\" cy=\"17\" r=\"2\"/>", "href": "#/settings", "nav": "settings", "kbd": "g s"},
  {"t": "sep"},
  {"t": "button", "title": "Command palette (⌘K)", "label": "Command", "svg": "<path d=\"M9 6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3z\"/>", "id": "palBtn", "kbd": "⌘K"},
  {"t": "button", "title": "Rail labels ([)", "label": "Collapse", "svg": "<rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/><path d=\"M9 4v16\"/>", "id": "railBtn", "kbd": "["},
];
