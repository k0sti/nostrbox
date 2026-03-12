export type View = "dashboard" | "registrations" | "actors" | "groups" | "relays";

interface LeftNavProps {
  active: View;
  onNavigate: (view: View) => void;
}

const NAV_ITEMS: { view: View; icon: string; label: string }[] = [
  { view: "dashboard", icon: "📊", label: "Dashboard" },
  { view: "registrations", icon: "📝", label: "Registrations" },
  { view: "actors", icon: "👥", label: "Actors" },
  { view: "groups", icon: "📁", label: "Groups" },
  { view: "relays", icon: "🔌", label: "Relays" },
];

export function LeftNav({ active, onNavigate }: LeftNavProps) {
  return (
    <nav className="left-nav">
      {NAV_ITEMS.map((item) => (
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
