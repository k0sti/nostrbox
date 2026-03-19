/**
 * ContextVM operation client.
 *
 * Uses @contextvm/sdk Nostr transport when connected, falls back to HTTP.
 */

import { callOpNostr, isTransportConnected } from "./contextvm";
import { loadSettings } from "./settings";

const API_BASE = import.meta.env.VITE_API_URL ?? "";

export interface OperationRequest {
  op: string;
  params?: Record<string, unknown>;
  caller?: string;
}

export interface OperationResponse<T = unknown> {
  ok: boolean;
  data?: T;
  error?: string;
  error_code?: string;
}

/** Currently authenticated pubkey, set after login for HTTP caller identity. */
let currentCaller: string | null = null;

export function setCurrentCaller(pubkey: string | null) {
  currentCaller = pubkey;
}

export async function callOp<T = unknown>(
  op: string,
  params: Record<string, unknown> = {}
): Promise<OperationResponse<T>> {
  // Use Nostr transport if connected and transport mode is "cvm"
  const settings = loadSettings();
  if (settings.transport === "cvm" && isTransportConnected()) {
    console.debug(`[nostrbox] ${op} → CVM (Nostr)`);
    return callOpNostr<T>(op, params);
  }

  if (settings.transport === "cvm") {
    console.warn(`[nostrbox] ${op} → CVM selected but transport not connected, falling back to HTTP`);
  }

  // HTTP fallback — include caller for auth-gated operations
  const body: OperationRequest = { op, params };
  if (currentCaller) {
    body.caller = currentCaller;
  }
  const res = await fetch(`${API_BASE}/api/op`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  return res.json();
}

// ── Typed operation helpers ────────────────────────────────

export interface DashboardSummary {
  pending_registrations: number;
  total_actors: number;
  total_groups: number;
  actors_by_role: Record<string, number>;
}

export interface Registration {
  pubkey: string;
  message: string | null;
  timestamp: number;
  status: "pending" | "approved" | "denied";
}

export interface Actor {
  pubkey: string;
  npub: string;
  kind: "human" | "agent" | "service" | "system";
  global_role: "guest" | "member" | "admin" | "owner";
  status: "active" | "disabled" | "banned" | "restricted";
  display_name: string | null;
  groups: string[];
  created_at: number;
  updated_at: number;
}

export interface ActorGroupEntry {
  group_id: string;
  group_name: string;
  role: "member" | "admin" | "owner";
}

export interface ActorDetail extends Actor {
  group_details: ActorGroupEntry[];
  registration_status: "pending" | "approved" | "denied" | null;
}

export interface Group {
  group_id: string;
  name: string;
  description: string;
  visibility: "public" | "group" | "internal";
  slug: string | null;
  join_policy: "open" | "request" | "invite_only" | "closed";
  status: "active" | "frozen" | "archived";
  members: GroupMember[];
  created_at: number;
  updated_at: number;
}

export interface GroupMember {
  pubkey: string;
  role: "member" | "admin" | "owner";
}

export interface RelayInfo {
  relay_url: string;
  pubkey: string;
  status: string;
  name?: string;
  description?: string;
  supported_nips?: number[];
  software?: string;
  version?: string;
}

export async function fetchRelayInfo(): Promise<RelayInfo | null> {
  try {
    const res = await fetch(`${API_BASE}/api/relay-info`, {
      headers: { Accept: "application/nostr+json" },
    });
    return res.json();
  } catch {
    return null;
  }
}

export const ops = {
  dashboardGet: () => callOp<DashboardSummary>("dashboard.get"),
  registrationSubmit: (pubkey: string, message?: string) =>
    callOp<Registration>("registration.submit", { pubkey, message }),
  registrationList: () => callOp<Registration[]>("registration.list"),
  registrationGet: (pubkey: string) =>
    callOp<Registration>("registration.get", { pubkey }),
  registrationApprove: (pubkey: string) =>
    callOp<Registration>("registration.approve", { pubkey }),
  registrationDeny: (pubkey: string) =>
    callOp<Registration>("registration.deny", { pubkey }),
  actorList: () => callOp<Actor[]>("actor.list"),
  actorGet: (pubkey: string) => callOp<Actor>("actor.get", { pubkey }),
  actorDetail: (pubkey: string) =>
    callOp<ActorDetail>("actor.detail", { pubkey }),
  groupList: () => callOp<Group[]>("group.list"),
  groupGet: (groupId: string) => callOp<Group>("group.get", { group_id: groupId }),
  groupPut: (group: Record<string, unknown>) => callOp<Group>("group.put", group),
  groupAddMember: (groupId: string, pubkey: string, role = "member") =>
    callOp("group.add_member", { group_id: groupId, pubkey, role }),
  groupRemoveMember: (groupId: string, pubkey: string) =>
    callOp("group.remove_member", { group_id: groupId, pubkey }),

  // Email login operations
  emailRegister: (npub: string, ncryptsec: string, email: string) =>
    callOp<{ status: string }>("email.register", { npub, ncryptsec, email }),
  emailLogin: (email: string) =>
    callOp<{ status: string }>("email.login", { email }),
  emailRedeem: (token: string) =>
    callOp<{ npub: string; ncryptsec: string }>("email.redeem", { token }),
  emailClear: () =>
    callOp<{ status: string }>("email.clear"),
  emailChangePassword: (ncryptsec: string) =>
    callOp<{ status: string }>("email.change_password", { ncryptsec }),
};
