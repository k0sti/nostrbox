import { useState } from "react";
import "./App.css";
import { TopBar } from "./components/TopBar";
import { LeftNav, type View } from "./components/LeftNav";
import { LoginModal } from "./components/LoginModal";
import { ProfilePanel } from "./components/ProfilePanel";
import { RelayPanel } from "./components/RelayPanel";
import { DashboardView } from "./views/DashboardView";
import { RegistrationsView } from "./views/RegistrationsView";
import { ActorsView } from "./views/ActorsView";
import { GroupsView } from "./views/GroupsView";
import { RelaysView } from "./views/RelaysView";
import { loginWithExtension, type NostrIdentity } from "./lib/nostr";

type Modal = "login" | "profile" | "relay" | null;

function App() {
  const [view, setView] = useState<View>("dashboard");
  const [modal, setModal] = useState<Modal>(null);
  const [identity, setIdentity] = useState<NostrIdentity | null>(null);

  const handleLoginExtension = async () => {
    const id = await loginWithExtension();
    if (id) {
      setIdentity(id);
      setModal(null);
    }
  };

  const handleLoginAmber = () => {
    // TODO: Implement Amber login
    console.warn("Amber login not yet implemented");
  };

  const handleLoginNostrConnect = () => {
    // TODO: Implement Nostr Connect login flow
    console.warn("Nostr Connect login not yet implemented");
  };

  const handleLogout = () => {
    setIdentity(null);
    setModal(null);
  };

  const renderView = () => {
    switch (view) {
      case "dashboard":
        return <DashboardView />;
      case "registrations":
        return <RegistrationsView />;
      case "actors":
        return <ActorsView />;
      case "groups":
        return <GroupsView />;
      case "relays":
        return <RelaysView />;
    }
  };

  return (
    <div className="app-layout">
      <TopBar
        identity={identity}
        onLoginClick={() => setModal("login")}
        onProfileClick={() => setModal("profile")}
        onRelayClick={() => setModal("relay")}
      />
      <div className="app-body">
        <LeftNav active={view} onNavigate={setView} />
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
  );
}

export default App;
