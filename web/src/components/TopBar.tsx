import type { NostrIdentity } from "../lib/nostr";

interface TopBarProps {
  identity: NostrIdentity | null;
  onLoginClick: () => void;
  onProfileClick: () => void;
  onRelayClick: () => void;
}

export function TopBar({
  identity,
  onLoginClick,
  onProfileClick,
  onRelayClick,
}: TopBarProps) {
  return (
    <header className="topbar">
      <div className="topbar-left">
        <button className="nostr-btn" onClick={onRelayClick}>
          Nostr
        </button>
      </div>
      <div className="topbar-center">Nostrbox</div>
      <div className="topbar-right">
        {identity ? (
          <button className="avatar-btn" onClick={onProfileClick}>
            {identity.picture ? (
              <img src={identity.picture} alt="" />
            ) : (
              "👤"
            )}
          </button>
        ) : (
          <button className="avatar-btn" onClick={onLoginClick}>
            👤
          </button>
        )}
      </div>
    </header>
  );
}
