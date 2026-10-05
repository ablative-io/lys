/** The one set of symbols an act is drawn with. Each is drawn here, on the rail's 24-point grid and in its line;
 *  an act that has no symbol gets one added to this set, never drawn where it is used. */
import type { ReactNode } from 'react';

const solid = { fill: 'currentColor', stroke: 'none' } as const;

export const SYMBOLS = {
  add: <path d="M12 5v14M5 12h14" />,
  remove: <path d="M5 12h14" />,
  save: <path d="M5 5h11l3 3v11H5zM8.5 5v4.5h6V5M8 19v-6h8v6" />,
  close: <path d="M6 6l12 12M18 6L6 18" />,
  stop: <rect x="6" y="6" width="12" height="12" rx="1.5" {...solid} />,
  start: <path d="M8 5.5v13l10-6.5z" {...solid} />,
  approve: <path d="M5 12.5l4.5 4.5L19 7.5" />,
  decline: <><circle cx="12" cy="12" r="8" /><path d="M6.5 17.5l11-11" /></>,
  retire: <><rect x="4" y="5" width="16" height="4" rx="1" /><path d="M6 9v9a1 1 0 0 0 1 1h10a1 1 0 0 0 1-1V9M10 13h4" /></>,
  suspend: <path d="M9 6v12M15 6v12" />,
  revoke: <path d="M9 7l-4 4 4 4M5 11h9a5 5 0 0 1 0 10h-3" />,
  copy: <><rect x="9" y="9" width="11" height="11" rx="1.5" /><path d="M5 15V5.5A1.5 1.5 0 0 1 6.5 4H15" /></>,
  edit: <path d="M4 20l1-4L16.5 4.5a2.1 2.1 0 0 1 3 3L8 19zM14.5 6.5l3 3" />,
  retry: <path d="M19 12a7 7 0 1 1-2.05-4.95M19 5v4h-4" />,
  again: <path d="M5 9a7 7 0 0 1 12.5-2.5L19 8M19 4v4h-4M19 15a7 7 0 0 1-12.5 2.5L5 16M5 20v-4h4" />,
  open: <path d="M14 5h5v5M19 5l-8 8M11 6H6.5A1.5 1.5 0 0 0 5 7.5v10A1.5 1.5 0 0 0 6.5 19h10a1.5 1.5 0 0 0 1.5-1.5V13" />,
  send: <path d="M4 12l16-7-6 16-3-6.5zM11 14.5L20 5" />,
  fit: <path d="M4 9V5h4M16 5h4v4M20 15v4h-4M8 19H4v-4" />,
  back: <path d="M14.5 6l-6 6 6 6" />,
  help: <><circle cx="12" cy="12" r="8" /><path d="M9.8 9.6a2.3 2.3 0 1 1 3.4 2c-.8.5-1.2 1-1.2 1.9M12 16.4v.1" /></>,
  watch: <><path d="M3 12s3.3-6 9-6 9 6 9 6-3.3 6-9 6-9-6-9-6z" /><circle cx="12" cy="12" r="2.6" /></>,
  enter: <path d="M13 5h5a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1h-5M4 12h10M10.5 8.5L14 12l-3.5 3.5" />,
  more: <><circle cx="6" cy="12" r="1.4" {...solid} /><circle cx="12" cy="12" r="1.4" {...solid} /><circle cx="18" cy="12" r="1.4" {...solid} /></>,
} as const satisfies Record<string, ReactNode>;

export type SymbolName = keyof typeof SYMBOLS;
