import { useMemo, useState } from 'react';
import { Act } from '../../shell/Act';
import { useShell } from '../../shell/ShellContext';
import { Delegate } from '../grants/Delegate';
import { ActPanel } from '../grants/GrantTable';
import { grantNo, onText, passesToAgents, type GrantWorld } from '../grants/model';
import type { Entry } from '../people/directory';
import { giveLink } from './giveLink';

/** An unavailable recipient still opens an empty drawer, never an unrelated form. */
export function MissingGive({ id, reason }: { id: string; reason: string }) {
  const [open, setOpen] = useState(true);
  return open ? (
    <ActPanel label="Give" opener={null} close={() => setOpen(false)}>
      <p role="alert">
        This agent does not exist among the records you can see: {id}. {reason} Nothing was filled.
      </p>
      <Act symbol="close" name="Cancel" word="Cancel" onClick={() => setOpen(false)} />
    </ActPanel>
  ) : null;
}

/** The same Give form whether opened by a press or filled by a reviewed link. */
export function GiveAccess({ x, w, query, reload }: { x: Entry; w: GrantWorld; query: string; reload: () => void }) {
  const shell = useShell();
  const linked = useMemo(() => giveLink(query, w, x.id), [query, w, x.id]);
  const [giving, setGiving] = useState(linked.at !== 'absent');
  const [opener, setOpener] = useState<HTMLElement | null>(null);
  const [sourceId, setSourceId] = useState(linked.at === 'ready' ? linked.source.id : '');
  const [fromLink, setFromLink] = useState(linked.at !== 'absent');
  const passable = w.list.grants.filter((grant) => grant.holder === w.me.person.id && grant.standing.stands && passesToAgents(grant.pass_on));
  const source = passable.find((grant) => grant.id === sourceId);
  const mineToGive = x.kind === 'agent' && x.person?.id === w.me.person.id && x.state !== 'retired';
  const refused = fromLink && linked.at === 'refused';
  return (
    <>
      <div className="file-section-head">
        <span className="section-h">Grants</span>
        {x.kind === 'agent' && x.state !== 'retired' ? (
          <Act
            symbol="send"
            name="Give access"
            word="Give"
            data-act="grant"
            onClick={(event) => {
              if (mineToGive && passable.length) {
                setSourceId('');
                setFromLink(false);
                setOpener(event.currentTarget);
                setGiving(true);
              } else
                shell.toast(
                  mineToGive
                    ? 'You hold no access you can give an agent. The button below records directory access only; Cambium needs a separate grant.'
                    : `Only ${x.display_name}’s responsible person can give it access.`
                );
            }}
          />
        ) : null}
      </div>
      {giving ? (
        <ActPanel label="Give" opener={opener} close={() => setGiving(false)}>
          {fromLink && linked.at === 'ready' ? (
            <p role="status">
              this link filled this form. Recipient: {x.display_name} ({x.id}). Resource kind: {linked.resource.kind}. Id (store): {linked.resource.id}.
              Actions: {linked.actions.join(', ')}. Read the choices below, then press Give.
            </p>
          ) : null}
          {refused ? (
            <>
              <p role="alert">{linked.reason}</p>
              <Act symbol="close" name="Cancel" word="Cancel" onClick={() => setGiving(false)} />
            </>
          ) : (
            <>
              <label className="field">
                Access to give
                <select
                  aria-label="Grant to give"
                  value={source?.id ?? ''}
                  onChange={(event) => {
                    setSourceId(event.target.value);
                    setFromLink(false);
                  }}>
                  <option value="">Choose access to give</option>
                  {passable.map((grant) => (
                    <option key={grant.id} value={grant.id}>
                      {onText(grant)} · {grant.relation} · {grantNo(grant.id)}
                    </option>
                  ))}
                </select>
              </label>
              {source ? (
                <Delegate
                  key={source.id}
                  w={w}
                  source={source}
                  to={x.id}
                  initialChoices={fromLink && linked.at === 'ready' ? linked.choices : []}
                  done={reload}
                  close={() => setGiving(false)}
                />
              ) : (
                <p className="hint">Choose a grant to see which actions you can give.</p>
              )}
            </>
          )}
        </ActPanel>
      ) : null}
    </>
  );
}
