/**
 * happy-dom can expose a `window` without a usable `localStorage` (opaque
 * origin). The app treats storage as best-effort, so tests get a real in-memory
 * implementation instead of monkey-patching each test file.
 */
class MemoryStorage implements Storage {
  private data = new Map<string, string>();

  get length(): number {
    return this.data.size;
  }

  clear(): void {
    this.data.clear();
  }

  getItem(key: string): string | null {
    return this.data.has(key) ? this.data.get(key)! : null;
  }

  key(index: number): string | null {
    return [...this.data.keys()][index] ?? null;
  }

  removeItem(key: string): void {
    this.data.delete(key);
  }

  setItem(key: string, value: string): void {
    this.data.set(key, String(value));
  }
}

function installStorage(target: object): void {
  const existing = (target as { localStorage?: Storage }).localStorage;
  if (existing) {
    try {
      const probe = "__drivedesk_probe__";
      existing.setItem(probe, "1");
      existing.removeItem(probe);
      return;
    } catch {
      // fall through to the in-memory implementation
    }
  }
  Object.defineProperty(target, "localStorage", { value: new MemoryStorage(), configurable: true });
}

installStorage(globalThis);
if (typeof window !== "undefined" && window !== globalThis) installStorage(window);
