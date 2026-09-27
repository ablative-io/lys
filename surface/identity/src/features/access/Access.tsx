import { useNavigate, useParams } from 'react-router';
import { useLoad } from '../../api';
import { keyable } from '../../shell/keyable';
import { Gate } from '../signin/Gate';
import { reachMap } from '../grants/check';
import { CheckBox, resourcesSeen } from '../grants/CheckBox';
import { grantNo, lastUsedText, lastsText, nameOf, onText, passText, readGrantWorld, resourceLabel, standing } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import { Pill } from '../people/Pill';

interface AccessData {
  w: GrantWorld;
  reach: Map<string, Map<string, string[]>>;
}

async function readAccess(): Promise<AccessData> {
  const w = await readGrantWorld();
  return { w, reach: await reachMap([...resourcesSeen(w).values()]) };
}

const person = (w: GrantWorld, id: string) => {
  const x = w.who.get(id);
  return { id, display_name: x?.name ?? nameOf(w, id), state: x?.state ?? 'active' } as const;
};

function Reach({ d, id }: { d: AccessData; id: string }) {
  const rows = [...d.reach].map(([res, byHolder]) => [res, byHolder.get(id) ?? []] as const).filter(([, acts]) => acts.length);
  return rows.length ? (
    <table><tbody>
      {rows.map(([res, acts]) => (
        <tr key={res}><td>{res}</td><td><span className="svc built-in">built in</span></td><td className="mono">{acts.join(', ')}</td><td /></tr>
      ))}
    </tbody></table>
  ) : <div className="dim">Nothing.</div>;
}

function Body({ d, mode, arg }: { d: AccessData; mode: string; arg?: string }) {
  const navigate = useNavigate();
  const { w } = d;
  const segs: [string, string][] = [['can', 'Can someone…'], ['reach', 'What can someone reach'], ['who', 'Who can reach something']];
  const ids = [...w.who.keys()];
  const resources = [...d.reach.keys()];
  const labels = new Map([...resourcesSeen(w)].map(([k, v]) => [k, resourceLabel(v.resource)]));
  let q = <CheckBox w={w} />;
  if (mode === 'reach') {
    const id = arg && w.who.has(arg) ? arg : w.me.person.id;
    q = (
      <>
        <div className="q" style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <span className="sec">What can</span>
          <select aria-label="Who" value={id} onChange={(e) => navigate('/access/reach/' + e.target.value)}>
            {ids.map((x) => <option key={x} value={x}>{nameOf(w, x)}</option>)}
          </select>
          <span className="sec">reach?</span>
        </div>
        <div className="card" style={{ marginTop: 12 }}><Reach d={d} id={id} /></div>
      </>
    );
  }
  if (mode === 'who') {
    const res = arg && d.reach.has(arg) ? arg : resources[0] ?? '';
    const holders = [...(d.reach.get(res) ?? new Map<string, string[]>())];
    q = (
      <>
        <div className="q" style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <span className="sec">Who can reach</span>
          <select aria-label="Resource" value={res} onChange={(e) => navigate('/access/who/' + e.target.value)}>
            {resources.map((r) => <option key={r} value={r}>{labels.get(r) ?? r}</option>)}
          </select>
          <span className="sec">?</span>
        </div>
        <div className="card" style={{ marginTop: 12 }} id="whoCan">
          {holders.length ? holders.map(([h, acts]) => (
            <div className="row" key={h}><span><Pill x={person(w, h)} /></span><span className="mono">{acts.join(', ')}</span></div>
          )) : <div className="dim">Nobody.</div>}
        </div>
      </>
    );
  }
  return (
    <div className="page">
      <div className="eyebrow">Access</div>
      <h1>Access</h1>
      <p className="sub">Ask it any way round. Every answer traces to a person, or says why not.</p>
      <div className="seg" style={{ marginBottom: 14 }}>
        {segs.map(([k, l]) => <a key={k} href={'#/access/' + k} className={mode === k ? 'on' : ''}>{l}</a>)}
      </div>
      <div className="check" style={{ marginTop: 0 }}>{q}</div>
      <div className="section-h"><span>Every grant</span><span className="open-q">draft form</span></div>
      <table>
        <thead><tr><th>Grant</th><th>Holder</th><th>Relation</th><th>On</th><th>Derives from</th><th>May pass on</th><th>Lasts</th><th>Last used</th><th>Stands</th></tr></thead>
        <tbody>
          {w.list.grants.map((g) => {
            const s = standing(w, g);
            const up = g.source ? w.byId.get(g.source) : undefined;
            const open = () => navigate(`/file/${g.holder}/access`);
            return (
              <tr key={g.id} data-href={`#/file/${g.holder}/access`} onClick={open} {...keyable(open)}>
                <td className="mono">{grantNo(g.id)}</td>
                <td>{nameOf(w, g.holder)}</td>
                <td className="mono">{g.relation}</td>
                <td className="mono">{onText(g)}</td>
                <td className="sec">{g.source ? `${grantNo(g.source)} · ${up ? nameOf(w, up.holder) : 'not visible'}` : <span className="dim">root</span>}</td>
                <td className="sec">{passText(g.pass_on)}</td>
                <td className="sec">{lastsText(g)}</td>
                <td className="sec">{g.last_use.seen ? lastUsedText(g) : <span className="dim">{lastUsedText(g)}</span>}</td>
                <td>{s.ok ? <><span className="dot s-active" />yes</> : <span style={{ color: 'var(--danger)' }} title={s.why}>no</span>}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
      {w.list.grants.length ? null : <div className="dim" style={{ marginTop: 10 }}>No grant you can see.</div>}
      <p className="note" style={{ marginTop: 8 }}>Last used is an exercise seen where access is enforced. Not seen means none was observed, never that it was never used.</p>
    </div>
  );
}

export function Access() {
  const { mode = 'can', arg } = useParams();
  const load = useLoad(readAccess, 'access');
  return <Gate load={load} title="Access" ok={(d) => <Body d={d} mode={mode} arg={arg} />} />;
}
