# Manifest v1 Dokümantasyonu

Canonical sözleşme schemas/patch-manifest-v1.schema.json dosyasındadır.

## Üst bölümler

- schema_version: Loader'ın desteklediği sözleşme sürümü. v1 değeri 1'dir.
- game: Sayısal ID, slug ve görünen ad.
- detection: Steam/Epic kimliği, executable, process ve kontrol dosyaları.
- patch: Patch ID/version, oyun sürümü, minimum loader, mandatory ve kanal.
- archive: Download-token endpoint'i, SHA-256 ve tam byte boyutu.
- install_actions: Sıralı allow-list dosya işlemleri.
- integrity: Kurulum sonrası doğrulama ve abort conflict politikası.
- backup: Otomatik backup ve uninstall'a kadar saklama.

## Action ortak alanları

- id: UUID v4 benzeri benzersiz action kimliği.
- type: İzinli sekiz türden biri.
- source: COPY işlemlerinde ZIP köküne; MOVE/RENAME işlemlerinde game root'a göre relative.
- destination: Her zaman game root'a göre relative.
- backup: Admin niyeti; destructive durumda motor yine güvenli backup alır.
- expected_sha256: İsteğe bağlı kaynak/hedef bütünlük beklentisi.
- options: Geleceğe dönük, action handler'ın açıkça tanıdığı seçenekler.

## Fail-closed davranış

Bilinmeyen schema veya action tipi reddedilir. Loader, server doğrulamış olsa bile manifesti tekrar doğrular. JSON'a sonradan shell benzeri alan eklemek davranış üretmez ve additionalProperties kontrolünde reddedilir.

## Path semantiği

Geçerli:

~~~text
Content/Localization/Game.locres
files/Turkish/localization.dat
~~~

Geçersiz:

~~~text
../../Windows/System32
C:\Windows
\server\share
/etc/passwd
Content/../outside
~~~

Rust tarafı lexical kontrolün ardından mevcut en yakın ancestor'u canonicalize eder. Böylece game root içindeki symlink/reparse üzerinden dışarı çıkış da reddedilir.

## Sürüm geçişi

Loader schema v1'i doğrudan deserialize eder. Gelecekte v2 geldiğinde v1 değiştirilmeyecek; ayrı v1-to-v2 saf migration fonksiyonu ve iki sürüm için fixture testleri eklenecektir. Desteklenmeyen gelecek sürüm sessizce yorumlanmaz.

## Installation manifest

Server manifestinden farklı olarak lokal installation kaydı gerçekleşen değişiklikleri içerir:

~~~json
{
  "game_id": 1,
  "patch_version": "1.0.0",
  "backup_id": "uuid",
  "active": true,
  "changes": [
    {
      "kind": "replaced_file",
      "path": "Localization/translation.dat",
      "backup_path": "files/uuid",
      "original_sha256": "...",
      "installed_sha256": "..."
    }
  ]
}
~~~

Uninstall önce installed_sha256 değerlerini mevcut dosyalarla karşılaştırır. Uyuşmazlıkta kullanıcı değişikliğini ezmez ve conflict raporu üretir.

