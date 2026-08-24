# Değişiklik Günlüğü

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
