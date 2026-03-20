import type { NostrIdentity } from "../lib/nostr";

export type View = "dashboard" | "register" | "registrations" | "actors" | "email-accounts" | "groups" | "agents" | "relays" | "events";

interface LeftNavProps {
  active: View;
  onNavigate: (view: View) => void;
  open?: boolean;
  identity: NostrIdentity | null;
}

interface NavItem {
  view: View;
  icon: string;
  label: string;
  /** Only visible when logged in as member or above */
  requiresMember?: boolean;
  /** Only visible when logged in as admin or owner */
  requiresAdmin?: boolean;
  /** Only visible when NOT logged in or not yet a member */
  publicOnly?: boolean;
}

const NAV_ITEMS: NavItem[] = [
  { view: "dashboard", icon: "📊", label: "Dashboard" },
  { view: "register", icon: "📋", label: "Register", publicOnly: true },
  { view: "registrations", icon: "📝", label: "Registrations", requiresAdmin: true },
  { view: "actors", icon: "👥", label: "Actors", requiresAdmin: true },
  { view: "email-accounts", icon: "📧", label: "Email Accounts", requiresAdmin: true },
  { view: "groups", icon: "📁", label: "Groups", requiresMember: true },
  { view: "agents", icon: "🤖", label: "Agents", requiresMember: true },
  { view: "relays", icon: "🔌", label: "Relays", requiresMember: true },
  { view: "events", icon: "📡", label: "Events", requiresMember: true },
];

const ADMIN_ROLES = ["admin", "owner"];
const MEMBER_ROLES = ["member", "admin", "owner"];

export function LeftNav({ active, onNavigate, open, identity }: LeftNavProps) {
  const role = identity?.role;
  const isAdmin = !!role && ADMIN_ROLES.includes(role);
  const isMember = !!role && MEMBER_ROLES.includes(role);

  const items = NAV_ITEMS.filter((item) => {
    if (item.requiresAdmin && !isAdmin) return false;
    if (item.requiresMember && !isMember) return false;
    if (item.publicOnly && isMember) return false;
    return true;
  });

  return (
    <nav className={`left-nav ${open ? "open" : ""}`}>
      {items.map((item) => (
        <button
          key={item.view}
          className={`nav-item ${active === item.view ? "active" : ""}`}
          onClick={() => onNavigate(item.view)}
        >
          <span className="nav-icon">{item.icon}</span>
          {item.label}
        </button>
      ))}
    </nav>
  );
}
