import {describe, expect, it} from "vitest";
import {compareVersions, hasNewerVersion, meetsMinimum} from "./version";

describe("compareVersions", () => {
  it("sıralamayı sayısal yapar, sözlük sırasına düşmez", () => {
    expect(compareVersions("1.10.0", "1.9.0")).toBe(1);
    expect(compareVersions("0.9.9", "1.0.0")).toBe(-1);
    expect(compareVersions("2.0.0", "2.0.0")).toBe(0);
  });

  it("eksik parçaları sıfır sayar", () => {
    expect(compareVersions("1.0", "1.0.0")).toBe(0);
    expect(compareVersions("1.0.1", "1.0")).toBe(1);
  });

  it("pre-release ve build ekini yok sayar", () => {
    expect(compareVersions("1.2.0-beta.3", "1.2.0")).toBe(0);
    expect(compareVersions("1.2.0+build9", "1.2.0")).toBe(0);
  });

  it("bozuk girdide çökmez", () => {
    expect(compareVersions("", "1.0.0")).toBe(-1);
    expect(compareVersions("abc", "0.0.0")).toBe(0);
  });
});

describe("meetsMinimum", () => {
  it("minimum tanımlı değilse engellemez", () => {
    expect(meetsMinimum("0.1.0", null)).toBe(true);
    expect(meetsMinimum("0.1.0", "   ")).toBe(true);
  });

  it("eski loader'ı engeller, eşit ve yeni sürüme izin verir", () => {
    expect(meetsMinimum("1.1.9", "1.2.0")).toBe(false);
    expect(meetsMinimum("1.2.0", "1.2.0")).toBe(true);
    expect(meetsMinimum("1.2.1", "1.2.0")).toBe(true);
  });
});

describe("hasNewerVersion", () => {
  it("sunucu sürümü daha yeniyse güncelleme vardır", () => {
    expect(hasNewerVersion("1.0.0", "1.0.1")).toBe(true);
  });

  it("sunucu sürümü eskiyse veya eşitse güncelleme yoktur", () => {
    expect(hasNewerVersion("1.0.1", "1.0.0")).toBe(false);
    expect(hasNewerVersion("1.0.0", "1.0.0")).toBe(false);
  });

  it("kurulu değilse veya sunucuda yama yoksa güncelleme göstermez", () => {
    expect(hasNewerVersion(null, "1.0.0")).toBe(false);
    expect(hasNewerVersion("1.0.0", null)).toBe(false);
  });
});
