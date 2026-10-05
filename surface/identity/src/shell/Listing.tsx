/**
 * The one list every screen uses: searched, grouped by team, each group opening with what it holds,
 * a page of rows at a time. It scrolls inside its own pane; the page around it never scrolls.
 */
import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import type { Group } from './org';
import { keyable } from './keyable';
import { count, counted } from './count';
import { Act } from './Act';

export interface Column<T> { head: string; cell: (item: T) => ReactNode }

export interface ListingProps<T> {
  groups: Group<T>[];
  columns: Column<T>[];
  id: (item: T) => string;
  /** Where a row opens, for the keyboard and for links. */
  href: (item: T) => string;
  /** The words a search is matched against. */
  words: (item: T) => string;
  /** What the rows are, plural: "people and agents", "computers". One of them is said in the singular. */
  noun: string;
  /** What the table body says when there is nothing to list, said once and only there. */
  empty?: ReactNode;
  /** What a group holds, after its name: "12 people, 48 agents". */
  holds: (items: T[]) => string;
  /** Rows folded under a row, such as a person's agents. */
  under?: (item: T) => T[];
  underNoun?: (count: number) => string;
  /** The heading of the column that opens a row's folded rows. */
  underHead?: string;
  selected: string | null;
  select: (item: T) => void;
  open?: (item: T) => void;
  /** Controls beside the search box. */
  tools?: ReactNode;
  /** The table's own last rows, such as the row that adds one: rows in the table's columns, kept last and shown even when the list is empty. */
  foot?: ReactNode;
  /** Reports the addresses shown, in order, for keyboard movement. */
  shown?: (ids: string[]) => void;
}

const PAGE = 25;

export function Listing<T>(props: ListingProps<T>) {
  const { groups, columns, id, href, words, noun, empty, holds, under, underNoun, underHead, selected, select, open, tools, foot, shown } = props;
  const [query, setQuery] = useState('');
  const [folded, setFolded] = useState<Set<string>>(() => new Set());
  const [opened, setOpened] = useState<Set<string>>(() => new Set());
  const [pages, setPages] = useState<Map<string, number>>(() => new Map());
  const needle = query.trim().toLowerCase();
  const hits = (item: T) => !needle || words(item).toLowerCase().includes(needle);
  const matching = groups.map((group) => ({
    ...group,
    items: group.items.filter((item) => hits(item) || (under?.(item).some(hits) ?? false)),
  })).filter((group) => !needle || group.items.length);
  const distinct = new Set(groups.flatMap((group) => group.items.flatMap((item) => [id(item), ...(under?.(item) ?? []).map(id)])));
  const found = new Set(matching.flatMap((group) => group.items.flatMap((item) => [item, ...(under?.(item) ?? [])]).filter(hits).map(id)));
  const toggle = (set: Set<string>, key: string) => { const next = new Set(set); if (next.has(key)) next.delete(key); else next.add(key); return next; };
  const width = columns.length + (under ? 1 : 0);
  const visible: string[] = [];
  const body = matching.map((group) => {
    const shut = folded.has(group.id) && !needle;
    const limit = pages.get(group.id) ?? PAGE;
    const page = shut ? [] : group.items.slice(0, limit);
    const small = group.items.reduce((sum, item) => sum + 1 + (under?.(item).length ?? 0), 0) <= PAGE;
    const isOpen = (item: T) => small !== opened.has(group.id + ':' + id(item));
    const rows = page.flatMap((item) => {
      const children = under?.(item) ?? [];
      const showChildren = children.filter((child) => needle ? hits(child) : isOpen(item));
      return [{ item, depth: 0, children }, ...showChildren.map((child) => ({ item: child, depth: 1, children: [] as T[] }))];
    });
    for (const row of rows) visible.push(href(row.item));
    return <tbody key={group.id || 'none'}>
      {matching.length === 1 && !group.id ? null : <tr className="group"><th colSpan={width}>
        <button className="group-name" aria-expanded={!shut} style={{ paddingLeft: group.depth * 14 }} onClick={() => setFolded((set) => toggle(set, group.id))}>{group.name}</button>
        <span className="note">{group.lead ? 'led by ' + group.lead + ' · ' : ''}{holds(group.within)}</span>
      </th></tr>}
      {rows.map(({ item, depth, children }) => {
        const key = id(item);
        const fold = group.id + ':' + key;
        return <tr key={fold + depth} data-href={href(item)} data-pick={visible.indexOf(href(item))} className={key === selected ? 'cursor' : ''} {...keyable(() => open?.(item))}
          onFocus={() => key !== selected && select(item)}>
          {columns.map((column, index) => [
            <td key={column.head} style={index === 0 && depth ? { paddingLeft: 28 } : undefined}>{column.cell(item)}</td>,
            index === 0 && under ? <td key="under">{children.length && !needle ? <button className="btn under" aria-expanded={isOpen(item)} onClick={(event) => { event.stopPropagation(); setOpened((set) => toggle(set, fold)); }}>{underNoun?.(children.length) ?? count(children.length)}</button> : null}</td> : null,
          ])}
        </tr>;
      })}
      {!shut && group.items.length > limit ? <tr className="more"><td colSpan={width}>
        <Act symbol="add" name={'Show ' + count(Math.min(PAGE, group.items.length - limit)) + ' more of ' + count(group.items.length - limit) + ' in ' + group.name} word={'Show ' + count(Math.min(PAGE, group.items.length - limit)) + ' more'} onClick={() => setPages((map) => new Map(map).set(group.id, limit + PAGE))} />
      </td></tr> : null}
    </tbody>;
  });
  const key = visible.join(',');
  useEffect(() => { shown?.(visible); }, [key]);
  return <div className="listing">
    <div className="tools">
      <input className="search" type="search" aria-label={'Search ' + noun} placeholder={distinct.size ? 'Search ' + counted(distinct.size, noun) : 'Search ' + noun} value={query} onChange={(event) => setQuery(event.target.value)} />
      {tools}
      {/* An empty list says so once, in the table body; the count says nothing then. */}
      {distinct.size ? <span className="note count">{needle ? count(found.size) + ' of ' + count(distinct.size) + ' match' : counted(distinct.size, noun) + (groups.length > 1 ? ' in ' + counted(groups.length, 'groups') : '')}</span> : null}
    </div>
    <div className="pane">
      <table>
        <thead><tr>{columns.map((column, index) => [<th key={column.head}>{column.head}</th>, index === 0 && under ? <th key="under">{underHead ?? ''}</th> : null])}</tr></thead>
        {body}
        {(needle ? matching.length : distinct.size) ? null : <tbody><tr className="empty"><td colSpan={width} className="dim">{needle ? 'Nothing matches “' + query.trim() + '”.' : empty ?? 'Nothing here yet.'}</td></tr></tbody>}
        {foot ? <tfoot>{foot}</tfoot> : null}
      </table>
    </div>
  </div>;
}
