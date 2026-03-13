/** Persistent app settings stored in localStorage. */

const SETTINGS_KEY = "nostrbox_settings";

export interface AppSettings {
  /** Transport mode: "http" uses REST API, "cvm" uses ContextVM over Nostr. */
  transport: "http" | "cvm";
  /** Custom relay address (empty string = use server default). */
  relayUrl: string;
}

const DEFAULTS: AppSettings = {
  transport: "http",
  relayUrl: "",
};

export function getDefaults(): AppSettings {
  return { ...DEFAULTS };
}

export function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return getDefaults();
    return { ...DEFAULTS, ...JSON.parse(raw) };
  } catch {
    return getDefaults();
  }
}

export function saveSettings(settings: AppSettings) {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}

export function resetSettings() {
  localStorage.removeItem(SETTINGS_KEY);
}
