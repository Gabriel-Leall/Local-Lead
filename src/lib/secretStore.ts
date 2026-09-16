import { load } from "@tauri-apps/plugin-store";

const FILE = "secrets.dat";
const KEY = "google_places_api_key";

// SecretStore abstraction — today backed by tauri-plugin-store,
// swappable for OS keychain later without touching UI.
class SecretStore {
  private store: Awaited<ReturnType<typeof load>> | null = null;

  private async getStore() {
    if (!this.store) {
      this.store = await load(FILE, { autoSave: true });
    }
    return this.store;
  }

  async getApiKey(): Promise<string> {
    try {
      const s = await this.getStore();
      const v = await s.get<string>(KEY);
      return v ?? "";
    } catch {
      return "";
    }
  }

  async setApiKey(key: string): Promise<void> {
    const s = await this.getStore();
    await s.set(KEY, key.trim());
    await s.save();
  }

  async clear(): Promise<void> {
    const s = await this.getStore();
    await s.delete(KEY);
    await s.save();
  }
}

export const secretStore = new SecretStore();
