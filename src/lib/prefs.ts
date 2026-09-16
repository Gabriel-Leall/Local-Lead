import { load } from "@tauri-apps/plugin-store";

const FILE = "ui.dat";

type Prefs = {
  lastQuery?: string;
  lastCity?: string;
  lastRadius?: number;
};

class PrefsStore {
  private store: Awaited<ReturnType<typeof load>> | null = null;

  private async getStore() {
    if (!this.store) {
      this.store = await load(FILE, { autoSave: true });
    }
    return this.store;
  }

  async load(): Promise<Prefs> {
    try {
      const s = await this.getStore();
      return {
        lastQuery: (await s.get<string>("lastQuery")) ?? undefined,
        lastCity: (await s.get<string>("lastCity")) ?? undefined,
        lastRadius: (await s.get<number>("lastRadius")) ?? undefined,
      };
    } catch {
      return {};
    }
  }

  async save(p: Prefs): Promise<void> {
    try {
      const s = await this.getStore();
      if (p.lastQuery !== undefined) await s.set("lastQuery", p.lastQuery);
      if (p.lastCity !== undefined) await s.set("lastCity", p.lastCity);
      if (p.lastRadius !== undefined) await s.set("lastRadius", p.lastRadius);
      await s.save();
    } catch {
      /* ignore */
    }
  }
}

export const prefs = new PrefsStore();
