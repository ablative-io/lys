import { api, request, useLoad } from "../api";
import { entries } from "../features/people/directory";
import type { NetworkView } from "../features/network/contract";

export async function readRecordNames(): Promise<Map<string, string>> {
  const [people, network] = await Promise.all([api.people(), request<NetworkView>("/network")]);
  return new Map([
    ...entries(people).map((entry) => [entry.id, entry.display_name] as const),
    ...network.machines.map((machine) => [machine.id, machine.name] as const),
  ]);
}
export function useRecordNames() {
  return useLoad(readRecordNames, "surface-record-names");
}
export function RecordName({ id, name }: { id: string; name?: string }) {
  return (
    <span className="record-name" title={id}>
      {name ?? id}
    </span>
  );
}
