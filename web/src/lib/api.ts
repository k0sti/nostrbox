/**
 * ContextVM operation client.
 *
 * TODO: Replace with web ContextVM SDK once integrated.
 * This is a placeholder HTTP client that mirrors the ContextVM operation pattern.
 */

const API_BASE = import.meta.env.VITE_API_URL ?? "http://localhost:3000";

export interface OperationRequest {
  op: string;
  params?: Record<string, unknown>;
  caller?: string;
}

export interface OperationResponse<T = unknown> {
  ok: boolean;
  data?: T;
  error?: string;
}

export async function callOp<T = unknown>(
  op: string,
  params: Record<string, unknown> = {}
): Promise<OperationResponse<T>> {
  const res = await fetch(`${API_BASE}/api/op`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ op, params } satisfies OperationRequest),
  });
  return res.json();
}

// ── Typed operation helpers ────────────────────────────────

export interface DashboardSummary {
  pending_registrations: number;
  total_actors: number;
  total_groups: number;
}

export interface Registration {
  pubkey: string;
  message: string | null;
  timestamp: number;
  status: "pending" | "approved" | "denied";
}

export interface Actor {
  pubkey: string;
  kind: "human" | "agent" | "service" | "system";
  global_role: "guest" | "member" | "admin" | "owner";
  display_name: string | null;
  groups: string[];
}

export interface Group {
  group_id: string;
  name: string;
  description: string;
  visibility: "public" | "group" | "internal";
  members: GroupMember[];
}

export interface GroupMember {
  pubkey: string;
  role: "member" | "admin" | "owner";
}

export const ops = {
  dashboardGet: () => callOp<DashboardSummary>("dashboard.get"),
  registrationList: () => callOp<Registration[]>("registration.list"),
  registrationGet: (pubkey: string) =>
    callOp<Registration>("registration.get", { pubkey }),
  registrationApprove: (pubkey: string) =>
    callOp<Registration>("registration.approve", { pubkey }),
  actorList: () => callOp<Actor[]>("actor.list"),
  actorGet: (pubkey: string) => callOp<Actor>("actor.get", { pubkey }),
  groupList: () => callOp<Group[]>("group.list"),
  groupGet: (groupId: string) => callOp<Group>("group.get", { group_id: groupId }),
  groupPut: (group: Omit<Group, "members">) => callOp<Group>("group.put", group as Record<string, unknown>),
  groupAddMember: (groupId: string, pubkey: string, role = "member") =>
    callOp("group.add_member", { group_id: groupId, pubkey, role }),
  groupRemoveMember: (groupId: string, pubkey: string) =>
    callOp("group.remove_member", { group_id: groupId, pubkey }),
};
