# Loader Sağlamlaştırma — Ne Değişti, Neden

Bu doküman `0.2.0` sürümündeki değişiklikleri açıklar. İki taraf da elden geçti:
**backend** (silme altyapısı) ve **loader istemcisi** (gerçek kurulum durumu,
güncelleme akışı, sürüm kapısı).

Doğrulama: 59 uçtan uca PHP testi + 6 güvenlik/manifest testi (gerçek MariaDB 10.11
üzerinde), 37 TypeScript testi, temiz `tsc` ve üretim `vite build`.

---

## 1. Kurulum

```bash
# Backend
php web/scripts/migrate.php          # 007 (idempotent hale getirildi) + 008 uygulanır
mkdir -p web/storage/trash && chmod 750 web/storage/trash

# .env'e ekle
#   TRASH_STORAGE_PATH=storage/trash
#   TRASH_RETENTION_DAYS=14

# Gece bakımı için cPanel cron
/usr/local/bin/php /home/KULLANICI/web/scripts/maintenance.php

# Loader
cd loader && npm ci
npm test && npx tsc --noEmit
VITE_API_BASE_URL=https://api.yusufgulsafak.com npm run build
npm run tauri build
```

`007_external_patch_sources.sql` daha önce çalıştıysa sorun yok: dosya artık
idempotent, kolon varsa atlıyor.

---

## 2. En kritik hata: güncelleme kurulumu oyunu bozuyordu

`patch/engine.rs::install()` her kurulumda **yeni ve boş bir backup** açıp
dokunduğu dosyaların o anki halini "orijinal" olarak kaydediyordu. Kurulu bir
yamanın üzerine yeni sürüm kurulduğunda sıra şuydu:

1. v1.0 kurulur → yedek: **vanilla** dosyalar. Doğru.
2. v1.1 kurulur → yedek: **v1.0'ın yamalı dosyaları**. Yanlış.
3. Kullanıcı "Yamayı Kaldır" der → oyun **v1.0 haline** döner, vanilla'ya değil.

Üstüne, `installations/<game_id>.json` üzerine yazıldığı için v1.0'ın kurulum
kaydı kayboluyordu. O yedeğin `metadata.json`'ı `active: true` kalıyor ve
`clean()` aktif yedekleri silmeyi reddettiği için o klasör **sonsuza kadar**
diskte kalıyordu.

Artık `install()`, arşivi indirip çıkardıktan **sonra**, oyun klasörüne
dokunmadan **önce** şunu yapıyor:

- Aynı oyun için aktif kurulum var mı? Varsa aynı klasörde mi?
- Önceki kurulumun dosyaları hâlâ beklenen SHA-256'da mı?
- Evetse yedekten geri yükle, kaydı kapat, sonra yeni sürümü uygula.
- Dosyalar elle değiştirilmişse kullanıcıya sorulur; onaylarsa `force` ile devam.

Regresyon testi: `src-tauri/src/patch/engine.rs` içindeki
`update_over_existing_install_restores_vanilla_first`.

---

## 3. Kurulu yama bilgisi artık gerçek

Önce:

```ts
localStorage.setItem("installed_" + game.id, String(game.patch_version));
```

Bunun üç sorunu vardı:

1. Webview verisi temizlenince loader tüm yamaları "kurulu değil" sanıyordu.
   Kullanıcı yamayı **kaldıramıyordu** — çünkü Kaldır butonu bu değere bakıyordu.
2. `game.patch_version` `null` ise `"null"` string'i yazılıyordu.
3. Oyun klasörü silinse/taşınsa bile "Kurulu" görünüyordu.

Artık kaynak Rust'ın `installations/<game_id>.json` journal'ı:

- Yeni komutlar: `list_installations`, `installation_for_game`.
- `InstallationRegistry` (`src/services/installations.ts`) bunu okur.
- Oyun klasörü kayıpsa kart "Oyun klasörü bulunamadı" rozetiyle gösterilir.
- Yedeği kaybolmuş kurulumlar açılışta uyarı olarak bildirilir.
- Güncelleme rozeti artık string eşitsizliği yerine sürüm karşılaştırması
  kullanıyor: sunucudaki sürüm **daha yeniyse** güncelleme gösterilir.

---

## 4. Ölü kod canlandırıldı

| Alan | Durum | Şimdi |
|---|---|---|
| `selfUpdate.ts` | Hiç çağrılmıyordu | Açılışta kontrol, güncelleme bandı, tek tıkla kurulum |
| `minimum_loader_version` | Manifestte vardı, denetlenmiyordu | İstemcide **ve** Rust'ta kurulum öncesi kapı |
| `config.support_url` | Tıklayınca sadece metin gösteriyordu | Sistem tarayıcısında açılıyor |
| `discord/youtube/instagram/x` | Hiç kullanılmıyordu | Destek kartında buton olarak çıkıyor |
| "Şifremi Unuttum" | "kullanılamıyor" yazıyordu | Web sayfasına yönlendiriyor |
| `download.rs` resume | Her kurulumda yeni tempdir olduğu için asla tetiklenmiyordu | SHA-256 anahtarlı kalıcı önbellek, gerçekten devam ediyor |

Bağlantı açma için yeni Tauri eklentisi eklenmedi; `open_external` komutu
yalnız `https` kabul eden, shell'e argüman geçirmeyen bir Rust komutu.

---

## 5. Silme altyapısı

Ekran görüntüsündeki yama listesinde Builder / Yayınla / Rollback / Durum vardı
ama **Sil yoktu**. Sebep basit: `delete_patch_version` diye bir işlem hiç yazılmamıştı.

`patch_release_channels.active_patch_version_id` yabancı anahtarı `ON DELETE
RESTRICT` olduğu için yayındaki bir sürümü silmeye kalkmak doğrudan FK hatası
verirdi. Bu yüzden `DeletionService` şunları yapıyor:

- **Etki raporu** (`describe_deletion`): kaç sürüm, kaç MB arşiv, kaç indirme
  kaydı etkilenecek — onay penceresi gerçek rakamları gösterir.
- **Kanal devri**: aktif yayın silinirse kanal bir önceki yayınlanmış sürüme
  geçer, o da yoksa kanal kaydı kaldırılır. Loader istemcileri boşluğa düşmez.
- **Karantina**: dosyalar `storage/trash/{alan}/{tarih}/` altına taşınır.
- **Kuyruk**: diske erişilemezse `storage_gc_queue`, cron tekrar dener.
- **Geçmiş korunur**: `download_logs` silinmez, anonimleşir; silinen kaydın tam
  kopyası `audit_logs.before_json` içine yazılır.

Eklenen işlemler: `delete_patch_version`, `delete_loader_version`,
`delete_subscription`, `delete_user` (yalnız süper admin), ve `delete_game` /
`delete_category` artık her iki arayüzde.

---

## 6. İki controller birbirinden kaymıştı

`AdminController` (web paneli) ve `ApiController` (loader istemcisi) ayrı `match`
blokları tutuyordu ve listeler uyuşmuyordu:

- Yalnız web panelinde: `update_user`, `save_subscription`, `create_loader_version`,
  `save_loader_config`, `save_branding_media`, `reset_branding_media`
- Yalnız API'de: `delete_game`, `delete_category`, `inspect_external_patch`

Her ikisi de `app/Support/AdminActions.php` registry'sine yönlendirildi.
42 action, tek liste. Bundan sonra eklenen her işlem otomatik olarak iki
arayüzde de çıkar.

Loader içindeki yönetim ekranına ayrıca **Kullanıcılar**, **Abonelikler**,
**Loader Sürümleri** ve **Bakım** sekmeleri eklendi — bunlar daha önce yalnız
web panelinde vardı.

---

## 7. Değişen dosyalar

### Yeni
```
web/app/Services/DeletionService.php      Tüm silme mantığı, etki raporu, force
web/app/Services/StorageGc.php            Karantina, GC kuyruğu, orphan tarama
web/app/Support/AdminActions.php          Tek action registry (42 action)
web/database/008_deletion_infrastructure.sql
web/scripts/maintenance.php               Cron bakım aracı
loader/src/services/version.ts            SemVer karşılaştırma
loader/src/services/installations.ts      Kurulum kayıt defteri
loader/src/services/version.test.ts
loader/src/services/installations.test.ts
loader/src/admin/impact.test.ts
```

### Değişen
```
web/database/007_external_patch_sources.sql   Idempotent hale getirildi
web/app/Controllers/AdminController.php       Registry'ye yönlendirildi
web/app/Controllers/ApiController.php         Registry + harici indirme koruması
web/app/Services/AdminService.php             3 hata düzeltmesi + silme devri
web/app/Services/{Patch,Loader,Image,BrandingMedia}Storage.php  GC erişimcileri
web/app/Core/Database.php                     Eksik DB ayarında net hata
web/resources/views/admin.php                 Sil butonları + Bakım paneli
web/public/assets/admin.js, admin-extended.css
loader/src-tauri/src/lib.rs                   7 yeni komut
loader/src-tauri/src/patch/engine.rs          Güncelleme düzeltmesi, sürüm kapısı, force
loader/src-tauri/src/backup/mod.rs            Kurulum kaydı listesi, prune
loader/src-tauri/src/download.rs              Kalıcı, devam ettirilebilir önbellek
loader/src-tauri/src/models.rs                Yeni dönüş tipleri
loader/src/main.ts                            Journal, sürüm kapısı, self-update, linkler
loader/src/stores/app.ts, types.ts, services/patch.ts
loader/src/admin/panel.ts, types.ts, admin.css
loader/src/styles.css
```

---

## 8. Notlar

- `loader/vite.config.ts` production build'de `VITE_API_BASE_URL` değerini
  `https://api.yusufgulsafak.com` olmaya zorluyor. Bu bilinçli bir güvenlik
  kontrolü; repoyu başka bir alan adı için fork ederseniz burayı değiştirmeniz
  gerekir.
- `web/app/Controllers/Yedek_ApiController.php` (ayrı API yüklemesinde vardı)
  route'lanmayan ölü kod; bu repoda yok, öyle kalması iyi.
- `RateLimiter` her istekte süresi dolmuş kayıtları siliyor. Yoğunlukta darboğaz
  olur; bu temizliği bakım cron'una taşımak sonraki adım için mantıklı.
- Silinen kayıtlar `audit_logs.before_json` içinde duruyor. Panelden tek tıkla
  geri alma (çöp kutusu ekranı) henüz yok, sonraki adım için iyi bir aday.
