/**
 * Lucide-style stroked icons as raw SVG path data.
 * Ported from design_artifacts/shared.jsx so component code can render via
 * <Icon d={IC.search} /> with no per-icon component file.
 *
 * If the icon set grows past ~30 entries, swap in `lucide-svelte`.
 */
export const IC = {
  chevronR: "<polyline points='9 18 15 12 9 6' />",
  chevronD: "<polyline points='6 9 12 15 18 9' />",
  search: "<circle cx='11' cy='11' r='7'/><path d='m20 20-3.5-3.5'/>",
  plus: "<path d='M12 5v14M5 12h14'/>",
  folder:
    "<path d='M3 6.5A1.5 1.5 0 0 1 4.5 5h4l2 2h9A1.5 1.5 0 0 1 21 8.5v9A1.5 1.5 0 0 1 19.5 19h-15A1.5 1.5 0 0 1 3 17.5v-11Z'/>",
  collection:
    "<rect x='3' y='4' width='18' height='4' rx='1'/><rect x='3' y='10' width='18' height='4' rx='1'/><rect x='3' y='16' width='18' height='4' rx='1'/>",
  send: "<path d='m22 2-7 20-4-9-9-4 20-7Z'/>",
  lock: "<rect x='5' y='11' width='14' height='10' rx='2'/><path d='M8 11V7a4 4 0 0 1 8 0v4'/>",
  unlock:
    "<rect x='5' y='11' width='14' height='10' rx='2'/><path d='M8 11V7a4 4 0 0 1 8 0'/>",
  copy: "<rect x='9' y='9' width='12' height='12' rx='2'/><path d='M5 15V5a2 2 0 0 1 2-2h10'/>",
  more: "<circle cx='5' cy='12' r='1'/><circle cx='12' cy='12' r='1'/><circle cx='19' cy='12' r='1'/>",
  x: "<path d='M18 6 6 18M6 6l12 12'/>",
  check: "<path d='M20 6 9 17l-5-5'/>",
  filter: "<path d='M3 5h18l-7 9v6l-4-2v-4L3 5Z'/>",
  arrowR: "<path d='M5 12h14M13 6l6 6-6 6'/>",
  caret: "<polyline points='6 9 12 15 18 9' />",
  globe:
    "<circle cx='12' cy='12' r='9'/><path d='M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18'/>",
  sidebar:
    "<rect x='3' y='4' width='18' height='16' rx='2'/><path d='M9 4v16'/>",
  history:
    "<path d='M3 12a9 9 0 1 0 3-6.7L3 8'/><path d='M3 3v5h5'/><path d='M12 7v5l3 2'/>",
  refresh:
    "<path d='M21 12a9 9 0 1 1-3-6.7L21 8'/><path d='M21 3v5h-5'/>",
  settings:
    "<circle cx='12' cy='12' r='3'/><path d='M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z'/>",
  pencil:
    "<path d='M17 3a2.83 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z'/><path d='m15 5 4 4'/>",
  trash:
    "<path d='M3 6h18'/><path d='M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2'/><path d='M19 6 17.5 20a2 2 0 0 1-2 2h-7a2 2 0 0 1-2-2L5 6'/>",
} as const;

export type IconName = keyof typeof IC;
