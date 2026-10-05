/** The one menu in a page's head: the acts nobody presses every day (Tom, 5 October 2026: "the suspend and retire button should be under maybe the drop down"). Escape or a press elsewhere closes it; choosing sends nothing, it opens that act's form. */
import { useEffect, useRef, useState } from 'react';
import { Act } from '../../shell/Act';

export interface HeadAct { act: string; label: string; danger?: boolean;
  /** The mark a test or a walk finds the item by, when it is not the act's own word. */
  mark?: string }

export function HeadMenu({ items, chosen, choose }: { items: HeadAct[]; chosen: string | null; choose: (act: string) => void }) {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const away = (event: PointerEvent) => { if (!box.current?.contains(event.target as Node)) setOpen(false); };
    const key = (event: KeyboardEvent) => { if (event.key === 'Escape') setOpen(false); };
    document.addEventListener('pointerdown', away);
    document.addEventListener('keydown', key);
    return () => { document.removeEventListener('pointerdown', away); document.removeEventListener('keydown', key); };
  }, [open]);
  if (!items.length) return null;
  return <div className="head-menu" ref={box}>
    <Act symbol="more" name="More actions" aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen(!open)} />
    {open ? <div role="menu" className="head-menu-list">
      {items.map((item) => <button key={item.act} type="button" role="menuitem" className={'head-menu-item' + (item.danger ? ' danger' : '')} data-act={item.mark ?? item.act} aria-pressed={chosen === item.act} onClick={() => { setOpen(false); choose(item.act); }}>{item.label}</button>)}
    </div> : null}
  </div>;
}
