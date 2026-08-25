import {describe, expect, it, vi} from "vitest";
import {InstallationRegistry} from "./installations";
import type {InstallationSummary} from "../types";

function record(overrides: Partial<InstallationSummary> = {}): InstallationSummary {
  return {
    game_id: 1,
    game_name: "Demo",
    patch_id: 10,
    patch_version: "1.0.0",
    game_root: "C:/Games/Demo",
    backup_id: "backup-1",
    created_at: "2026-01-01T00:00:00Z",
    root_exists: true,
    backup_exists: true,
    change_count: 3,
    ...overrides,
  };
}

describe("InstallationRegistry", () => {
  it("kurulu sürümü journal'dan okur", async () => {
    const registry = new InstallationRegistry(async () => [record()]);
    await registry.refresh();
    expect(registry.version(1)).toBe("1.0.0");
    expect(registry.root(1)).toBe("C:/Games/Demo");
  });

  it("oyun klasörü kayıpsa kurulu saymaz ama kaydı gösterir", async () => {
    const registry = new InstallationRegistry(async () => [record({root_exists: false})]);
    await registry.refresh();
    expect(registry.version(1)).toBeNull();
    expect(registry.isOrphaned(1)).toBe(true);
    expect(registry.get(1)).not.toBeNull();
  });

  it("güncellemeyi sürüm sırasına göre belirler", async () => {
    const registry = new InstallationRegistry(async () => [record({patch_version: "1.0.0"})]);
    await registry.refresh();
    expect(registry.hasUpdate(1, "1.0.1")).toBe(true);
    expect(registry.hasUpdate(1, "1.0.0")).toBe(false);
    // Sunucudaki sürüm eskiyse güncelleme rozeti gösterilmemeli.
    expect(registry.hasUpdate(1, "0.9.0")).toBe(false);
  });

  it("yedeği kaybolmuş kurulumları listeler", async () => {
    const registry = new InstallationRegistry(async () => [
      record({game_id: 1}),
      record({game_id: 2, backup_exists: false}),
    ]);
    await registry.refresh();
    expect(registry.brokenBackups().map(item => item.game_id)).toEqual([2]);
  });

  it("journal okunamazsa akışı bozmaz", async () => {
    const failing = vi.fn(async () => {
      throw new Error("IPC yok");
    });
    const registry = new InstallationRegistry(failing);
    await expect(registry.refresh()).resolves.toBeUndefined();
    expect(registry.version(1)).toBeNull();
  });

  it("bir kez yüklendikten sonra hata gelirse son bilinen durumu korur", async () => {
    let fail = false;
    const registry = new InstallationRegistry(async () => {
      if (fail) throw new Error("geçici hata");
      return [record()];
    });
    await registry.refresh();
    fail = true;
    await registry.refresh();
    expect(registry.version(1)).toBe("1.0.0");
  });

  it("çıkışta temizlenir", async () => {
    const registry = new InstallationRegistry(async () => [record()]);
    await registry.refresh();
    registry.clear();
    expect(registry.all()).toEqual([]);
  });
});
