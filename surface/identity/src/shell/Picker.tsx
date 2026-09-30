/** Choose people or agents by name, however many there are: type, pick from the few that match, remove with ×. The choice travels in hidden inputs named `name`. */
import { useState } from 'react';

export interface Pickable { id: string; name: string; detail?: string }

export function Picker({ name, label, options, multiple = false, onChange }: { name: string; label: string; options: Pickable[]; multiple?: boolean; onChange?: (ids: string[]) => void }) {
  const [query, setQuery] = useState('');
  const [chosen, setChosen] = useState<Pickable[]>([]);
  const needle = query.trim().toLowerCase();
  const matches = needle ? options.filter((option) => option.name.toLowerCase().includes(needle) && !chosen.some((each) => each.id === option.id)).slice(0, 8) : [];
  const set = (next: Pickable[]) => { setChosen(next); onChange?.(next.map((each) => each.id)); };
  const pick = (option: Pickable) => { set(multiple ? [...chosen, option] : [option]); setQuery(''); };
  if (!options.length) return <span className="hint">There is no one to choose yet.</span>;
  return <div className="picker">
    {chosen.map((option) => <span key={option.id} className="pill">{option.name}<input type="hidden" name={name} value={option.id} />{' '}<button type="button" aria-label={'Remove ' + option.name} onClick={() => set(chosen.filter((each) => each.id !== option.id))}>×</button></span>)}
    {multiple || !chosen.length ? <input className="search" type="search" aria-label={label} placeholder={label + ' (' + options.length.toLocaleString('en-AU') + ')'} value={query} onChange={(event) => setQuery(event.target.value)}
      onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); if (matches[0]) pick(matches[0]); } }} /> : null}
    {matches.length ? <ul className="plain">{matches.map((option) => <li key={option.id}><button type="button" className="btn" onClick={() => pick(option)}>{option.name}</button>{option.detail ? <span className="note"> {option.detail}</span> : null}</li>)}</ul> : null}
  </div>;
}
