import {describe, expect, it, vi} from "vitest";

vi.mock("../api/client", () => ({api: {adminAction: vi.fn()}}));

const {humanBytes, impactText} = await import("./panel");
import type {DeletionReport} from "./types";

function report(overrides: Partial<DeletionReport> = {}): DeletionReport {
  return {
    entity: "patch_version",
    id: 5,
    label: "Demo 1.1.0 (stable)",
    blocking: [],
    cascade: {},
    requires_force: false,
    ...overrides,
  };
}

describe("humanBytes", () => {
  it("bayt değerlerini okunur birime çevirir", () => {
    expect(humanBytes(0)).toBe("0 B");
    expect(humanBytes(512)).toBe("512 B");
    expect(humanBytes(1024)).toBe("1.0 KB");
    expect(humanBytes(1073741824)).toBe("1.0 GB");
  });

  it("geçersiz girdide çökmez", () => {
    expect(humanBytes(null)).toBe("0 B");
    expect(humanBytes(undefined)).toBe("0 B");
  });
});

describe("impactText", () => {
  it("engelleyici uyarıları başa alır", () => {
    const text = impactText(report({blocking: ["Bu sürüm stable kanalının aktif yayını."]}));
    expect(text.startsWith("! Bu sürüm stable")).toBe(true);
  });

  it("oyun silmede sürüm ve arşiv etkisini yazar", () => {
    const text = impactText(
      report({
        entity: "game",
        cascade: {patch_versions: 5, published_versions: 2, archive_bytes: 1048576, download_logs_detached: 40},
      }),
    );
    expect(text).toContain("Silinecek yama sürümü: 5 (yayında: 2)");
    expect(text).toContain("1.0 MB");
    expect(text).toContain("İndirme kaydı korunacak (anonimleşir): 40");
  });

  it("kanal devrini gösterir", () => {
    const text = impactText(report({cascade: {is_active_release: true, replacement_version: "1.0.0"}}));
    expect(text).toContain("Kanal şu sürüme geri dönecek: 1.0.0");
    expect(text).not.toContain("yayında yama kalmayacak");
  });

  it("yerine geçecek sürüm yoksa uyarır", () => {
    const text = impactText(report({cascade: {is_active_release: true}}));
    expect(text).toContain("Bu kanalda yayında yama kalmayacak.");
  });

  it("etkisi olmayan silmede boş metin döner", () => {
    expect(impactText(report())).toBe("");
  });
});
