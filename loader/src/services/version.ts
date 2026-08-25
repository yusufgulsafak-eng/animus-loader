/**
 * Loader ve yama sürümleri SemVer biçiminde ("1.2.3", "1.2.3-beta.1").
 * Karşılaştırmada pre-release / build eki yok sayılır: yayın akışında
 * "1.2.0-rc.1" ile "1.2.0" aynı yeteneğe sahip kabul edilir.
 */
export function compareVersions(left: string, right: string): number {
  const parts = (value: string): number[] =>
    String(value ?? "")
      .trim()
      .split(/[-+]/)[0]
      .split(".")
      .map(piece => {
        const parsed = Number.parseInt(piece, 10);
        return Number.isFinite(parsed) ? parsed : 0;
      });

  const a = parts(left);
  const b = parts(right);
  for (let index = 0; index < Math.max(a.length, b.length); index += 1) {
    const x = a[index] ?? 0;
    const y = b[index] ?? 0;
    if (x !== y) return x > y ? 1 : -1;
  }
  return 0;
}

/** `current` sürümü `minimum` şartını karşılıyor mu? */
export function meetsMinimum(current: string, minimum: string | null | undefined): boolean {
  const required = String(minimum ?? "").trim();
  if (!required) return true;
  return compareVersions(current, required) >= 0;
}

/**
 * Kurulu yama ile sunucudaki yama arasında güncelleme var mı?
 * Sunucu sürümü eski ya da eşitse güncelleme yoktur; bu yüzden basit
 * eşitsizlik yerine sürüm karşılaştırması kullanılır.
 */
export function hasNewerVersion(installed: string | null | undefined, available: string | null | undefined): boolean {
  if (!installed || !available) return false;
  return compareVersions(available, installed) > 0;
}
