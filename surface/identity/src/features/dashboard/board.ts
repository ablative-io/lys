/** What the canvas's widgets read: the person's agents with what Lys holds about each, and what waits for the person. */
import { Refused, api, request } from '../../api';
import { readTogether } from '../../reads';
import { readDrafts } from '../drafts/contract';
import type { Draft } from '../drafts/contract';
import type { AccessRequest } from '../requests/contract';
import { readDashboard } from './contract';
import type { DashboardAgent } from './contract';

/** A part of a page that could not be read is its refusal; the rest of the page still reads. */
export const orRefused = <T,>(read: Promise<T>, name: string): Promise<T | Refused> =>
  read.then((value) => value, (problem: unknown) => problem instanceof Refused ? problem : new Refused(0, { refusal: name, reason: String(problem) }));

export interface Board { me: string; rows: DashboardAgent[]; requests: AccessRequest[] | Refused; drafts: Draft[] | Refused }

export async function readBoard(): Promise<Board> {
  const me = await api.me();
  const { answer, requests, drafts } = await readTogether({
    answer: readDashboard(),
    requests: orRefused<AccessRequest[]>(request<{ requests: AccessRequest[] }>('/requests').then((list) => list.requests), 'RequestsUnreadable'),
    drafts: orRefused(readDrafts('waiting'), 'DraftsUnreadable'),
  });
  return { me: me.person.id, rows: answer.agents.filter((row) => row.agent.state !== 'retired'), requests, drafts };
}
