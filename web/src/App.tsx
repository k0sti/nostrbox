import { useState, useEffect } from "react";
import "./App.css";
import { TopBar } from "./components/TopBar";
import { LeftNav, type View } from "./components/LeftNav";
import { LoginModal } from "./components/LoginModal";
import { ProfilePanel } from "./components/ProfilePanel";
import { RelayPanel } from "./components/RelayPanel";
import { DashboardView } from "./views/DashboardView";
import { RegisterView } from "./views/RegisterView";
import { RegistrationsView } from "./views/RegistrationsView";
import { ActorsView } from "./views/ActorsView";
import { GroupsView } from "./views/GroupsView";
import { RelaysView } from "./views/RelaysView";
import { EventsView } from "./views/EventsView";
import { loginWithExtension, loginWithAmber, loginWithNostrConnect, disconnectNostrConnect, fetchProfile, type NostrIdentity } from "./lib/nostr";
import { connectTransport, disconnectTransport } from "./lib/contextvm";
import { ops, setCurrentCaller } from "./lib/api";
import { loadSettings } from "./lib/settings";
import { RelayProvider } from "./lib/relay-context";

type Modal = "login" | "profile" | "relay" | null;

const ADMIN_ROLES = ["admin", "owner"];
const MEMBER_ROLES = ["member", "admin", "owner"];
const STORAGE_KEY = "nostrbox_identity";

/** Save minimal identity to localStorage (pubkey + npub only — role/profile refetched). */
function saveIdentity(id: NostrIdentity | null) {
  if (id) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ pubkey: id.pubkey, npub: id.npub }));
  } else {
    localStorage.removeItem(STORAGE_KEY);
  }
}

/** Load saved identity from localStorage. */
function loadIdentity(): NostrIdentity | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const { pubkey, npub } = JSON.parse(raw);
    if (pubkey && npub) return { pubkey, npub };
  } catch { /* ignore corrupt data */ }
  return null;
}

function App() {
  const [view, setView] = useState<View>("dashboard");
  const [modal, setModal] = useState<Modal>(null);
  const [identity, setIdentity] = useState<NostrIdentity | null>(null);
  const [navOpen, setNavOpen] = useState(false);

  const role = identity?.role;
  const isAdmin = !!role && ADMIN_ROLES.includes(role);
  const isMember = !!role && MEMBER_ROLES.includes(role);

  // Restore session on mount
  useEffect(() => {
    const saved = loadIdentity();
    if (saved) {
      hydrateIdentity(saved);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Redirect away from views the user can't access
  useEffect(() => {
    const adminViews: View[] = ["registrations", "actors"];
    const memberViews: View[] = ["groups", "relays"];
    const publicOnlyViews: View[] = ["register"];

    if (adminViews.includes(view) && !isAdmin) setView("dashboard");
    if (memberViews.includes(view) && !isMember) setView("dashboard");
    if (publicOnlyViews.includes(view) && isMember) setView("dashboard");
  }, [identity, view, isAdmin, isMember]);

  /** Hydrate an identity: set state, fetch role + profile metadata. */
  const hydrateIdentity = async (id: NostrIdentity) => {
    setIdentity(id);
    setCurrentCaller(id.pubkey);

    // Connect CVM transport first if configured, so subsequent ops use it
    const settings = loadSettings();
    if (settings.transport === "cvm") {
      await connectTransport().catch(() => {});
    }

    // Fetch actor role from backend
    ops.actorGet(id.pubkey).then((res) => {
      if (res.ok && res.data) {
        setIdentity((prev) =>
          prev ? { ...prev, role: res.data!.global_role, displayName: prev.displayName || res.data!.display_name || undefined } : prev
        );
      }
    }).catch(() => {});

    // Fetch kind-0 profile metadata (picture, display name) in background
    fetchProfile(id).then((updated) => {
      if (updated.picture || updated.displayName) {
        setIdentity((prev) => prev ? { ...prev, displayName: updated.displayName || prev.displayName, picture: updated.picture || prev.picture } : prev);
      }
    });
  };

  const onLogin = async (id: NostrIdentity) => {
    saveIdentity(id);
    setModal(null);
    hydrateIdentity(id);
  };

  const handleLoginExtension = async () => {
    const id = await loginWithExtension();
    if (id) onLogin(id);
  };

  const handleLoginAmber = async () => {
    const id = await loginWithAmber();
    if (id) onLogin(id);
  };

  const handleLoginNostrConnect = async (bunkerUrl: string) => {
    const id = await loginWithNostrConnect(bunkerUrl);
    if (id) onLogin(id);
  };

  const handleLogout = () => {
    saveIdentity(null);
    setIdentity(null);
    setModal(null);
    setView("dashboard");
    setCurrentCaller(null);
    disconnectTransport().catch(() => {});
    disconnectNostrConnect().catch(() => {});
  };

  const handleNavigate = (v: View) => {
    setView(v);
    setNavOpen(false);
  };

  const renderView = () => {
    switch (view) {
      case "dashboard":
        return <DashboardView />;
      case "register":
        return <RegisterView onLoginClick={() => setModal("login")} />;
      case "registrations":
        return <RegistrationsView />;
      case "actors":
        return <ActorsView />;
      case "groups":
        return <GroupsView />;
      case "relays":
        return <RelaysView />;
      case "events":
        return <EventsView />;
    }
  };

  return (
    <RelayProvider>
    <div className="app-layout">
      <TopBar
        identity={identity}
        onLoginClick={() => setModal("login")}
        onProfileClick={() => setModal("profile")}
        onSettingsClick={() => setModal("relay")}
        onMenuClick={() => setNavOpen(!navOpen)}
      />
      <div className="app-body">
        {navOpen && (
          <div className="nav-overlay open" onClick={() => setNavOpen(false)} />
        )}
        <LeftNav active={view} onNavigate={handleNavigate} open={navOpen} identity={identity} />
        <main className="content-panel">{renderView()}</main>
      </div>

      {modal === "login" && (
        <LoginModal
          onClose={() => setModal(null)}
          onLoginExtension={handleLoginExtension}
          onLoginAmber={handleLoginAmber}
          onLoginNostrConnect={handleLoginNostrConnect}
        />
      )}
      {modal === "profile" && identity && (
        <ProfilePanel
          identity={identity}
          onClose={() => setModal(null)}
          onLogout={handleLogout}
        />
      )}
      {modal === "relay" && <RelayPanel onClose={() => setModal(null)} />}
    </div>
    </RelayProvider>
  );
}

export default App;
