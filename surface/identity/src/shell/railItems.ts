// The rail, item for item as the mock-up draws it. Icon paths are its own.

export type RailItem =
  | { t: 'sep' }
  | { t: 'grow' }
  | { t: 'a' | 'button'; title: string; label: string; svg: string; href?: string; nav?: string; dock?: 'help' | 'assistant'; id?: string; kbd?: string; cnt?: string };

export const RAIL: RailItem[] = [
  {"t": "a", "title": "Team (g e)", "label": "Team", "svg": "<rect x=\"3\" y=\"5\" width=\"18\" height=\"14\" rx=\"2\"/><path d=\"M7 10l3 2-3 2M12.5 14.5H17\"/>", "href": "#/team", "nav": "team", "kbd": "g e"},
  {"t": "a", "title": "You (g u)", "label": "You", "svg": "<circle cx=\"12\" cy=\"8\" r=\"3.5\"/><path d=\"M5 20c1-4 3.8-6 7-6s6 2 7 6\"/>", "href": "#/me", "nav": "me", "kbd": "g u"},
  {"t": "a", "title": "People and agents (g p)", "label": "People and agents", "svg": "<circle cx=\"9\" cy=\"8\" r=\"3.2\"/><path d=\"M3.5 19c.8-3.2 3-5 5.5-5s4.7 1.8 5.5 5\"/><circle cx=\"17\" cy=\"9\" r=\"2.4\"/><path d=\"M15.5 14.2c2.3.2 4 1.8 4.6 4.8\"/>", "href": "#/people", "nav": "people", "kbd": "g p"},
  {"t": "a", "title": "Agent canvas (g c)", "label": "Agent canvas", "svg": "<rect x=\"3\" y=\"4\" width=\"7\" height=\"6\" rx=\"1.2\"/><rect x=\"14\" y=\"4\" width=\"7\" height=\"6\" rx=\"1.2\"/><rect x=\"8.5\" y=\"14\" width=\"7\" height=\"6\" rx=\"1.2\"/><path d=\"M10 7h4M6.5 10l3.5 4M17.5 10 14 14\"/>", "href": "#/canvas", "nav": "canvas", "kbd": "g c"},
  {"t": "a", "title": "Roles (g o)", "label": "Roles", "svg": "<rect x=\"4\" y=\"7\" width=\"16\" height=\"12\" rx=\"2\"/><path d=\"M9 7V5.5A1.5 1.5 0 0 1 10.5 4h3A1.5 1.5 0 0 1 15 5.5V7\"/><path d=\"M4 12h16\"/>", "href": "#/roles", "nav": "roles", "kbd": "g o"},
  {"t": "sep"},
  {"t": "a", "title": "Resources (g r)", "label": "Resources", "svg": "<path d=\"M12 3 20 7.5 12 12 4 7.5z\"/><path d=\"M4 12l8 4.5 8-4.5M4 16.5 12 21l8-4.5\"/>", "href": "#/resources", "nav": "resources", "kbd": "g r"},
  {"t": "a", "title": "Access (g a)", "label": "Access", "svg": "<circle cx=\"8\" cy=\"15\" r=\"3.5\"/><path d=\"M10.5 12.5 19 4M16 7l2.5 2.5M14 9l2 2\"/>", "href": "#/access", "nav": "access", "kbd": "g a"},
  {"t": "a", "title": "Graph (g h)", "label": "Graph", "svg": "<circle cx=\"6\" cy=\"7\" r=\"2\"/><circle cx=\"18\" cy=\"6\" r=\"2\"/><circle cx=\"12\" cy=\"17\" r=\"2\"/><circle cx=\"19\" cy=\"16\" r=\"1.6\"/><path d=\"M8 7.5l8-1M7 9l4 6.5M17 8l-4 7.5M14 17l3.5-.8\"/>", "href": "#/graph", "nav": "graph", "kbd": "g h"},
  {"t": "a", "title": "Requests (g q)", "label": "Requests", "svg": "<path d=\"M4 5h16v11H9l-5 4z\"/><path d=\"M12 8v3M12 13.5v.01\"/>", "href": "#/requests", "nav": "requests", "kbd": "g q", "cnt": "cntReq"},
  {"t": "a", "title": "Reviews (g w)", "label": "Reviews", "svg": "<rect x=\"5\" y=\"4\" width=\"14\" height=\"17\" rx=\"2\"/><path d=\"M9 4v2h6V4M8.5 12l2 2 4-4.5\"/>", "href": "#/reviews", "nav": "reviews", "kbd": "g w", "cnt": "cntRev"},
  {"t": "sep"},
  {"t": "a", "title": "Secrets (g v)", "label": "Secrets", "svg": "<rect x=\"4\" y=\"10\" width=\"16\" height=\"10\" rx=\"2\"/><path d=\"M8 10V7a4 4 0 0 1 8 0v3\"/><circle cx=\"12\" cy=\"15\" r=\"1.3\"/>", "href": "#/secrets", "nav": "secrets", "kbd": "g v"},
  {"t": "a", "title": "Connections (g n)", "label": "Connections", "svg": "<circle cx=\"6\" cy=\"12\" r=\"2.5\"/><circle cx=\"18\" cy=\"6\" r=\"2.5\"/><circle cx=\"18\" cy=\"18\" r=\"2.5\"/><path d=\"M8.3 11 15.7 7M8.3 13l7.4 4\"/>", "href": "#/connections", "nav": "connections", "kbd": "g n"},
  {"t": "a", "title": "Network (g t)", "label": "Network", "svg": "<rect x=\"3\" y=\"4\" width=\"7\" height=\"5\" rx=\"1\"/><rect x=\"14\" y=\"4\" width=\"7\" height=\"5\" rx=\"1\"/><rect x=\"8.5\" y=\"15\" width=\"7\" height=\"5\" rx=\"1\"/><path d=\"M6.5 9v3h11V9M12 12v3\"/>", "href": "#/network", "nav": "network", "kbd": "g t"},
  {"t": "a", "title": "Sessions (g x)", "label": "Sessions", "svg": "<path d=\"M4 12h3l2-5 4 10 2-5h5\"/>", "href": "#/sessions", "nav": "sessions", "kbd": "g x"},
  {"t": "sep"},
  {"t": "a", "title": "Model (g m)", "label": "Model", "svg": "<circle cx=\"6\" cy=\"6\" r=\"2.2\"/><circle cx=\"18\" cy=\"6\" r=\"2.2\"/><circle cx=\"12\" cy=\"18\" r=\"2.2\"/><path d=\"M8 6.5h8M7 8l4 8M17 8l-4 8\"/>", "href": "#/model", "nav": "model", "kbd": "g m"},
  {"t": "grow"},
  {"t": "button", "title": "Help (?)", "label": "Help", "svg": "<circle cx=\"12\" cy=\"12\" r=\"8.5\"/><path d=\"M9.6 9.5a2.5 2.5 0 1 1 3.4 2.3c-.6.3-1 .8-1 1.5v.4M12 16.8v.01\"/>", "dock": "help", "kbd": "?"},
  {"t": "button", "title": "Assistant", "label": "Assistant", "svg": "<rect x=\"5\" y=\"8\" width=\"14\" height=\"10\" rx=\"3\"/><path d=\"M12 4v4M9.5 12.5v.5M14.5 12.5v.5M3 13v2M21 13v2\"/>", "dock": "assistant"},
  {"t": "a", "title": "Configuration (g s)", "label": "Configuration", "svg": "<path d=\"M4 7h10M18 7h2M4 17h4M12 17h8\"/><circle cx=\"16\" cy=\"7\" r=\"2\"/><circle cx=\"10\" cy=\"17\" r=\"2\"/>", "href": "#/settings", "nav": "settings", "kbd": "g s"},
  {"t": "sep"},
  {"t": "button", "title": "Command palette (⌘K)", "label": "Command", "svg": "<path d=\"M9 6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3z\"/>", "id": "palBtn", "kbd": "⌘K"},
  {"t": "button", "title": "Rail labels ([)", "label": "Collapse", "svg": "<rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/><path d=\"M9 4v16\"/>", "id": "railBtn", "kbd": "["},
];
