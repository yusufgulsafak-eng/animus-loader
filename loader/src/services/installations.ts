import type {InstallationSummary} from "../types";
import {hasNewerVersion} from "./version";

/**
 * Kurulu yamaların tek doğruluk kaynağı.
 *
 * Önceden bu bilgi `localStorage["installed_<id>"]` içinde tutuluyordu.
 * Bunun üç sorunu vardı:
 *  1. Webview verisi temizlenince loader, kurulu yamaları "yok" sanıyordu ve
 *     kullanıcı yamayı bir daha kaldıramıyordu.
 *  2. Kurulum yarıda kalsa bile kayıt yazılabiliyordu; gerçekle uyuşmuyordu.
 *  3. Oyun klasörü silinse/taşınsa bile "Kurulu" görünüyordu.
 *
 * Artık kaynak, Rust tarafının `installations/<game_id>.json` journal'ıdır.
 */
export class InstallationRegistry {
  private byGame = new Map<number, InstallationSummary>();
  private loaded = false;

  constructor(private readonly fetchAll: () => Promise<InstallationSummary[]>) {}

  async refresh(): Promise<void> {
    try {
      const rows = await this.fetchAll();
      this.byGame = new Map(rows.map(row => [Number(row.game_id), row]));
      this.loaded = true;
    } catch {
      // Journal okunamıyorsa boş kabul et; kullanıcı akışı durmasın.
      if (!this.loaded) this.byGame = new Map();
    }
  }

  get(gameId: number): InstallationSummary | null {
    return this.byGame.get(Number(gameId)) ?? null;
  }

  /** Kurulu yama sürümü; oyun klasörü artık yoksa kurulu sayılmaz. */
  version(gameId: number): string | null {
    const record = this.get(gameId);
    if (!record || !record.root_exists) return null;
    return record.patch_version || null;
  }

  /** Kurulum kaydı var ama oyun klasörü kayıp — kullanıcıya uyarı gösterilir. */
  isOrphaned(gameId: number): boolean {
    const record = this.get(gameId);
    return Boolean(record && !record.root_exists);
  }

  /** Kurulumun yapıldığı oyun kökü. Manuel seçimden daha güvenilirdir. */
  root(gameId: number): string | null {
    return this.get(gameId)?.game_root ?? null;
  }

  hasUpdate(gameId: number, availableVersion: string | null | undefined): boolean {
    return hasNewerVersion(this.version(gameId), availableVersion);
  }

  /** Yedeği kaybolmuş kurulumlar: kaldırma işlemi vanilla'ya dönemez. */
  brokenBackups(): InstallationSummary[] {
    return [...this.byGame.values()].filter(record => !record.backup_exists);
  }

  all(): InstallationSummary[] {
    return [...this.byGame.values()];
  }

  clear(): void {
    this.byGame = new Map();
    this.loaded = false;
  }
}
