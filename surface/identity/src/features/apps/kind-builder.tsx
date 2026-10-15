import { useState } from 'react';
import { Act } from '../../shell/Act';
import { breakable } from '../grants/GrantTable';
import type { Draft, DraftKind } from './SchemaBuilder';

export function KindNode({ app, draft, kind, seen, change }: { app: string; draft: Draft; kind: DraftKind; seen: string[]; change: (edit: (next: Draft) => void) => void }) {
  const at = (next: Draft) => next.kinds.find((entry) => entry.name === kind.name);
  const children = draft.kinds.filter((other) => other.parents[0] === kind.name && !seen.includes(other.name));
  return (
    <div className="sb-node">
      <fieldset className="sb-kind" aria-label={'Kind ' + kind.name}>
        <legend>
          <span className="sb-prefix">{app}.</span>
          <input
            className="sb-name"
            aria-label={'Name of kind ' + kind.name}
            value={kind.name}
            onChange={(event) => {
              const name = event.target.value;
              change((next) => {
                for (const other of next.kinds) other.parents = other.parents.map((parent) => (parent === kind.name ? name : parent));
                const own = at(next);
                if (own) own.name = name;
              });
            }}
          />
        </legend>
        <div className="sb-row">
          <span className="note">Actions</span>
          {kind.actions.map((action) => (
            <span className="sb-chip" key={action}>
              <span className="sb-action-name">{breakable(action)}</span>
              <Act
                symbol="close"
                name={'Remove action ' + action + ' from ' + kind.name}
                onClick={() =>
                  change((next) => {
                    const own = at(next);
                    if (own) {
                      own.actions = own.actions.filter((held) => held !== action);
                      for (const relation of own.relations) relation.actions = relation.actions.filter((held) => held !== action);
                    }
                  })
                }
              />
            </span>
          ))}
          <AddName
            label={'Add an action to ' + kind.name}
            placeholder="action"
            onAdd={(name) =>
              change((next) => {
                const own = at(next);
                if (own && !own.actions.includes(name)) own.actions.push(name);
              })
            }
          />
        </div>
        <fieldset className="sb-row sb-agent-actions" aria-label={'Agent actions on ' + kind.name}>
          <legend className="note">Actions agents may hold</legend>
          {kind.actions.map((action) => (
            <label className="sb-chip sb-agent-action" key={action}>
              <input
                type="checkbox"
                className="sb-tick"
                aria-label={'Agents may hold ' + action + ' on ' + kind.name}
                checked={kind.agents?.includes(action) ?? false}
                onChange={(event) => {
                  const on = event.target.checked;
                  change((next) => {
                    const own = at(next);
                    if (own) own.agents = on ? [...(own.agents ?? []), action] : (own.agents ?? []).filter((held) => held !== action);
                  });
                }}
              />
              <span className="sb-action-name">{breakable(action)}</span>
            </label>
          ))}
          <span className="hint">Only checked actions may be held by agents. No actions checked means agents may hold none.</span>
        </fieldset>
        <table className="sb-relations">
          <thead>
            <tr>
              <th scope="col">Relation</th>
              {kind.actions.map((action) => (
                <th scope="col" key={action}>
                  <span className="sb-action-name">{breakable(action)}</span>
                </th>
              ))}
              <th />
            </tr>
          </thead>
          <tbody>
            {kind.relations.map((relation, index) => (
              <tr key={index}>
                <th scope="row">
                  <input
                    className="sb-name"
                    aria-label={'Name of relation ' + relation.name + ' on ' + kind.name}
                    value={relation.name}
                    onChange={(event) => {
                      const name = event.target.value;
                      change((next) => {
                        const own = at(next)?.relations[index];
                        if (own) own.name = name;
                      });
                    }}
                  />
                </th>
                {kind.actions.map((action) => (
                  <td key={action}>
                    <input
                      type="checkbox"
                      className="sb-tick"
                      aria-label={relation.name + ' carries ' + action}
                      checked={relation.actions.includes(action)}
                      onChange={(event) => {
                        const on = event.target.checked;
                        change((next) => {
                          const own = at(next)?.relations[index];
                          if (own) own.actions = on ? [...own.actions, action] : own.actions.filter((held) => held !== action);
                        });
                      }}
                    />
                  </td>
                ))}
                <td>
                  <Act
                    symbol="remove"
                    name={'Remove relation ' + relation.name + ' from ' + kind.name}
                    className="small"
                    onClick={() =>
                      change((next) => {
                        const own = at(next);
                        if (own) own.relations.splice(index, 1);
                      })
                    }
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <AddName
          label={'Add a relation to ' + kind.name}
          placeholder="relation"
          onAdd={(name) =>
            change((next) => {
              const own = at(next);
              if (own && !own.relations.some((relation) => relation.name === name)) own.relations.push({ name, actions: [] });
            })
          }
        />
        <div className="sb-row">
          <span className="note">Parents</span>
          {draft.kinds
            .filter((other) => other.name !== kind.name)
            .map((other) => (
              <label className="sb-chip" key={other.name}>
                <input
                  type="checkbox"
                  className="sb-tick"
                  aria-label={'Parent ' + other.name + ' of ' + kind.name}
                  checked={kind.parents.includes(other.name)}
                  onChange={(event) => {
                    const on = event.target.checked;
                    change((next) => {
                      const own = at(next);
                      if (own) own.parents = on ? [...own.parents, other.name] : own.parents.filter((held) => held !== other.name);
                    });
                  }}
                />
                {other.name}
              </label>
            ))}
        </div>
        <Act
          symbol="remove"
          name={'Remove kind ' + kind.name}
          word="Remove kind"
          tone="danger"
          className="small sb-remove"
          onClick={() =>
            change((next) => {
              next.kinds = next.kinds.filter((entry) => entry.name !== kind.name);
              for (const other of next.kinds) other.parents = other.parents.filter((parent) => parent !== kind.name);
            })
          }
        />
      </fieldset>
      {children.length ? (
        <div className="sb-children">
          {children.map((child) => (
            <KindNode key={draft.kinds.indexOf(child)} app={app} draft={draft} kind={child} seen={[...seen, kind.name]} change={change} />
          ))}
        </div>
      ) : null}
    </div>
  );
}

/** A name typed and added with its button; nothing is added while it is blank. */
export function AddName({ label, placeholder, onAdd }: { label: string; placeholder: string; onAdd: (name: string) => void }) {
  const [name, setName] = useState('');
  const add = () => {
    const trimmed = name.trim();
    if (trimmed) {
      onAdd(trimmed);
      setName('');
    }
  };
  return (
    <span className="sb-add">
      <input
        className="sb-name"
        aria-label={label}
        placeholder={placeholder}
        value={name}
        onChange={(event) => setName(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault();
            add();
          }
        }}
      />
      <Act symbol="add" name={label} className="small" onClick={add} disabled={!name.trim()} />
    </span>
  );
}
