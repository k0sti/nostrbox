import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { Relay } from "applesauce-relay";
import type { AuthSigner } from "applesauce-relay/types";
import { combineLatest } from "rxjs";
import { loadSettings } from "./settings";

export type RelayStatus = "disconnected" | "connecting" | "connected" | "authenticated" | "auth-failed" | "error";

interface RelayContextValue {
  relay: Relay | null;
  status: RelayStatus;
  url: string;
}

const RelayContext = createContext<RelayContextValue>({
  relay: null,
  status: "disconnected",
  url: "",
});

function getRelayUrl(): string {
  const settings = loadSettings();
  if (settings.relayUrl) return settings.relayUrl;
  const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${proto}//${window.location.host}/ws`;
}

function getNip07Signer(): AuthSigner | null {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const nostr = (window as any).nostr;
  if (!nostr?.signEvent) return null;
  return nostr as AuthSigner;
}

export function RelayProvider({ children }: { children: ReactNode }) {
  const url = useMemo(getRelayUrl, []);
  const [status, setStatus] = useState<RelayStatus>("connecting");

  const relay = useMemo(() => new Relay(url), [url]);

  useEffect(() => {
    // Trigger actual WebSocket connection by creating a subscription.
    // connected$ / authenticated$ alone won't open the socket (they're passive observers).
    // A minimal REQ keeps the WebSocket alive.
    const connSub = relay.subscription([{ limit: 0 }]).subscribe();

    // Watch connection + auth state
    const sub = combineLatest([relay.connected$, relay.authenticated$]).subscribe(
      ([connected, authenticated]) => {
        if (!connected) {
          setStatus("disconnected");
        } else if (authenticated) {
          setStatus("authenticated");
        } else {
          setStatus("connected");
        }
      }
    );

    // Watch auth failures
    const authRespSub = relay.authenticationResponse$.subscribe((resp) => {
      if (resp && !resp.ok) {
        setStatus("auth-failed");
      }
    });

    // Attempt NIP-42 auth when challenge arrives
    const challengeSub = relay.challenge$.subscribe((challenge) => {
      if (challenge) {
        const signer = getNip07Signer();
        if (signer) {
          relay.authenticate(signer).catch((e) => {
            console.warn("NIP-42 auth failed:", e);
            setStatus("auth-failed");
          });
        }
      }
    });

    return () => {
      connSub.unsubscribe();
      sub.unsubscribe();
      authRespSub.unsubscribe();
      challengeSub.unsubscribe();
      relay.close();
    };
  }, [relay]);

  return (
    <RelayContext.Provider value={{ relay, status, url }}>
      {children}
    </RelayContext.Provider>
  );
}

export function useRelay() {
  return useContext(RelayContext);
}
