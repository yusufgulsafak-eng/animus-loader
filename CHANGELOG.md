# Değişiklik Günlüğü

## 0.2.0 — 2026-08-25 · Silme altyapısı ve loader sağlamlaştırma

### Kritik düzeltmeler
- **Güncelleme kurulumu oyunu bozuyordu.** Kurulu bir yamanın üzerine yeni sürüm kurulduğunda yeni yedek, "orijinal" olarak zaten yamalı dosyaları kaydediyordu; bu yüzden "Yamayı Kaldır" oyunu hiçbir zaman gerçek vanilla haline döndüremiyordu. Ayrıca eski kurulum kaydı üzerine yazıldığı için eski yedek `active` işaretli kalıyor ve asla temizlenemiyordu. Artık yeni sürüm uygulanmadan önce önceki yama geri alınıyor.
- **`007_external_patch_sources.sql` düz `ALTER TABLE` idi.** Kolonları elle eklenmiş canlı veritabanlarında `migrate.php` "Duplicate column" hatasıyla duruyordu. Migration idempotent hale getirildi.
- **Kurulu yama bilgisi `localStorage`'da tutuluyordu.** Webview verisi temizlenince loader kurulu yamaları göremiyor ve kullanıcı yamayı kaldıramıyordu. Artık tek doğruluk kaynağı Rust tarafındaki kurulum journal'ı.
- **`minimum_loader_version` hiçbir yerde denetlenmiyordu.** Manifestteki alan ölü veriydi; artık kurulum öncesi hem istemcide hem Rust çekirdeğinde kontrol ediliyor.
- **`selfUpdate.ts` ölü koddu.** `checkForLoaderUpdate` hiç çağrılmıyordu; loader kendi güncellemesini kontrol etmiyordu. Açılışta kontrol ediliyor ve güncelleme bandı gösteriliyor.
- Destek ve sosyal medya bağlantıları tıklanınca hiçbir şey yapmıyordu; artık sistem tarayıcısında açılıyor. "Şifremi Unuttum" web sayfasına yönlendiriyor.

### Silme altyapısı
- `delete_patch_version`, `delete_loader_version`, `delete_subscription`, `delete_user` işlemleri hiç yoktu; eklendi.
- `delete_game` ve `delete_category` yalnız API'de vardı; artık her iki arayüzde de var.
- Silmeden önce sunucudan gerçek etki raporu alınıyor (`describe_deletion`): kaç sürüm, kaç MB arşiv, kaç indirme kaydı etkilenecek.
- Yayındaki kayıtlar ek onay olmadan silinemiyor; aktif yayın silinince kanal otomatik olarak bir önceki yayınlanmış sürüme devrediliyor.
- Silinen dosyalar `unlink` edilmiyor, `storage/trash` karantinasına alınıyor (varsayılan 14 gün).
- Diske erişilemezse kayıt `storage_gc_queue` tablosuna düşüyor ve bakım cron'unda tekrar deneniyor.
- Kendi hesabını silme, son admini silme ve yetkisiz süper admin silme engelleniyor.

### Mimari
- `AdminController` ve `ApiController` ayrı ayrı action listeleri tutuyor ve birbirinden kaymıştı (`update_user` yalnız panelde, `delete_game` yalnız API'de çalışıyordu). Tek `AdminActions` registry'sine indirildi: 42 action, iki arayüz.
- Loader içindeki yönetim ekranına Kullanıcılar, Abonelikler, Loader Sürümleri ve Bakım sekmeleri eklendi.
- İndirme artık SHA-256 ile anahtarlanmış kalıcı önbelleğe iniyor: bağlantı koparsa aynı dosyadan devam ediliyor.
- Sahipsiz yedekleri ve indirme önbelleğini temizleyen `prune_storage` komutu ve Yedekler ekranında karşılığı eklendi.
- `duplicateGame` `supported_stores` alanını çift JSON kodluyordu; düzeltildi, kategori kopyalama eklendi.
- `setPatchStatus` yayından düşen sürümü kanalın aktif yayını olarak bırakıyordu; artık devrediyor veya temizliyor.
- `scripts/maintenance.php` cron aracı eklendi.

## 0.1.0 — 2026-08-24

- Sıfırdan PHP 8/MySQL web, API ve admin mimarisi kuruldu.
- Normalize oyun, patch, sürüm, action, release, kullanıcı, abonelik, loader config, download ve audit şemaları eklendi.
- Üç özgün demo oyun ve patch şablonu seed edildi.
- Güvenli ilk-admin CLI aracı eklendi.
- Oyun CRUD/kopyalama, ZIP upload, SHA-256, action builder, manifest test/yayın ve remote loader config akışları eklendi.
- Manifest JSON Schema v1 ve örnek manifest eklendi.
- Tek generic Tauri/TypeScript loader arayüzü eklendi.
- Rust path guard, ZIP Slip koruması, Steam tespiti, download/hash, action registry, backup/journal/rollback/uninstall/verify motoru eklendi.
- Koyu mor/siyah, lime accent responsive admin ve loader tasarımı oluşturuldu.
- cPanel, Windows setup, loader build, admin, manifest ve ayrıntılı patch oluşturma dokümantasyonu eklendi.
- Gerçek MySQL kataloğuna verilen adlarıyla 38 oyun eklendi; katalog mock veriden ayrıldı.
- Admin oyun CRUD, cover/banner/icon upload, patch ZIP/action/manifest/publish ve rollback akışları tamamlandı.
- Login ve ana loader için bağımsız resim/video/default branding medyası, ayrı overlay, fallback, private storage, range streaming ve remote config eklendi.
- Loader'da tek aktif video, ekran geçişinde kaynak cleanup ve güvenli varsayılan fallback davranışı eklendi.
- Admin view veri aktarımındaki `EXTR_SKIP` isim çakışması giderildi.
- Loader giriş/kayıt/çıkış akışı tamamen uygulama içine alındı; harici browser auth yönlendirmeleri kaldırıldı.
- Access token saklama Windows Credential Manager tabanlı tek bir güvenli servis altında toplandı; merkezi API client Bearer header, oturum restore ve 401 reset davranışı kazandı.
- `/api/auth/register` başarılı kayıtta güvenli otomatik giriş için kullanıcı ve access token döndürecek şekilde mevcut auth servisi üzerinden genişletildi; `/api/auth/me` alias'ı eklendi.
- Production frontend build'i HTTPS `VITE_API_BASE_URL` olmadan hata verecek şekilde güvenli hale getirildi.
- Windows release build engeli Code Integrity/Smart App Control olaylarıyla doğrulandı; hiçbir Windows güvenlik politikası değiştirilmedi.
