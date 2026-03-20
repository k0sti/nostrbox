import { useState, useEffect } from "react";
import "./App.css";
import { TopBar } from "./components/TopBar";
import { LeftNav, type View } from "./components/LeftNav";
import { LoginModal, PasswordDecryptModal } from "./components/LoginModal";
import { ProfilePanel } from "./components/ProfilePanel";
import { RelayPanel } from "./components/RelayPanel";
import { DashboardView } from "./views/DashboardView";
import { RegisterView } from "./views/RegisterView";
import { RegistrationsView } from "./views/RegistrationsView";
import { ActorsView } from "./views/ActorsView";
import { GroupsView } from "./views/GroupsView";
import { RelaysView } from "./views/RelaysView";
import { EventsView } from "./views/EventsView";
import { AgentsView } from "./views/AgentsView";
import { EmailAccountsView } from "./views/EmailAccountsView";
import { loginWithExtension, loginWithAmber, loginWithNostrConnect, disconnectNostrConnect, fetchProfile, type NostrIdentity } from "./lib/nostr";
import { connectTransport, disconnectTransport } from "./lib/contextvm";
import { ops, setCurrentCaller } from "./lib/api";
import { loadSettings } from "./lib/settings";
import { RelayProvider } from "./lib/relay-context";
import { setLoginMethod, type LoginMethod } from "./lib/signer";
import { clearStoredNsec, getStoredNsec } from "./lib/nip49";

type Modal = "login" | "profile" | "relay" | "password-decrypt" | null;

const ADMIN_ROLES = ["admin", "owner"];
const MEMBER_ROLES = ["member", "admin", "owner"];
const STORAGE_KEY = "nostrbox_identity";
const LOGIN_METHOD_KEY = "nostrbox_login_method";

/** Save minimal identity to localStorage (pubkey + npub only — role/profile refetched). */
function saveIdentity(id: NostrIdentity | null, method?: LoginMethod) {
  if (id) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ pubkey: id.pubkey, npub: id.npub }));
    if (method) localStorage.setItem(LOGIN_METHOD_KEY, method);
  } else {
    localStorage.removeItem(STORAGE_KEY);
    localStorage.removeItem(LOGIN_METHOD_KEY);
  }
}

/** Load saved identity from localStorage. */
function loadIdentity(): { identity: NostrIdentity | null; method: LoginMethod | null } {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { identity: null, method: null };
    const { pubkey, npub } = JSON.parse(raw);
    const method = localStorage.getItem(LOGIN_METHOD_KEY) as LoginMethod | null;
    if (pubkey && npub) return { identity: { pubkey, npub }, method };
  } catch { /* ignore corrupt data */ }
  return { identity: null, method: null };
}

function App() {
  const [view, setView] = useState<View>("dashboard");
  const [modal, setModal] = useState<Modal>(null);
  const [identity, setIdentity] = useState<NostrIdentity | null>(null);
  const [navOpen, setNavOpen] = useState(false);

  // Magic link redemption data (token redeemed, waiting for password)
  const [redeemData, setRedeemData] = useState<{ npub: string; ncryptsec: string } | null>(null);
  const [tokenError, setTokenError] = useState<string | null>(null);

  const role = identity?.role;
  const isAdmin = !!role && ADMIN_ROLES.includes(role);
  const isMember = !!role && MEMBER_ROLES.includes(role);

  // Handle magic link token on mount
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const token = params.get("token");
    if (token) {
      // Clean URL
      window.history.replaceState({}, "", window.location.pathname);
      // Redeem the token
      ops.emailRedeem(token).then((res) => {
        if (res.ok && res.data) {
          setRedeemData(res.data);
          setModal("password-decrypt");
        } else {
          console.error("Token redemption failed:", res.error);
          setTokenError("Login link expired or invalid. Please request a new one.");
        }
      });
    }
  }, []);

  // Restore session on mount
  useEffect(() => {
    const { identity: saved, method } = loadIdentity();
    if (saved) {
      if (method) setLoginMethod(method);
      // For email login, check if nsec is still in sessionStorage
      if (method === "email" && !getStoredNsec()) {
        // Session expired — clear saved identity
        saveIdentity(null);
        return;
      }
      hydrateIdentity(saved);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Redirect away from views the user can't access
  useEffect(() => {
    const adminViews: View[] = ["registrations", "actors", "email-accounts"];
    const memberViews: View[] = ["groups", "agents", "relays"];
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

  const onLogin = async (id: NostrIdentity, method: LoginMethod = "extension") => {
    setLoginMethod(method);
    saveIdentity(id, method);
    setModal(null);
    hydrateIdentity(id);
  };

  const handleLoginExtension = async () => {
    const id = await loginWithExtension();
    if (id) onLogin(id, "extension");
  };

  const handleLoginAmber = async () => {
    const id = await loginWithAmber();
    if (id) onLogin(id, "amber");
  };

  const handleLoginNostrConnect = async (bunkerUrl: string) => {
    const id = await loginWithNostrConnect(bunkerUrl);
    if (id) onLogin(id, "nostr-connect");
  };

  const handleEmailLogin = (pubkey: string, npub: string, _nsecHex: string) => {
    const id: NostrIdentity = { pubkey, npub };
    onLogin(id, "email");
    setRedeemData(null);
  };

  const handleLogout = () => {
    clearStoredNsec();
    setLoginMethod(null);
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
      case "email-accounts":
        return <EmailAccountsView />;
      case "groups":
        return <GroupsView />;
      case "agents":
        return <AgentsView />;
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
      {tokenError && (
        <div style={{ background: "var(--danger)", color: "#fff", padding: "10px 16px", fontSize: 14, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span>{tokenError}</span>
          <button onClick={() => setTokenError(null)} style={{ background: "none", border: "none", color: "#fff", cursor: "pointer", fontSize: 18, lineHeight: 1, padding: "0 4px" }} aria-label="Dismiss">&times;</button>
        </div>
      )}
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
          onEmailLogin={handleEmailLogin}
        />
      )}
      {modal === "password-decrypt" && redeemData && (
        <PasswordDecryptModal
          redeemData={redeemData}
          onDecrypt={handleEmailLogin}
          onClose={() => { setModal(null); setRedeemData(null); }}
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
