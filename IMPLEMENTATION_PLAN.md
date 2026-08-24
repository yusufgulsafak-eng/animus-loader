# Türkçe Yama Platformu — Uygulama Planı

## 1. Amaç ve değişmez ilkeler

Bu repository sıfırdan kurulacaktır. Sistem; PHP 8/MySQL tabanlı web sitesi, REST API ve yönetim paneli ile Tauri/Rust/TypeScript tabanlı tek bir Windows loader'dan oluşacaktır. Oyunlar loader koduna yazılmayacak; oyun, tespit ve kurulum davranışı sunucunun yayınladığı sürümlenmiş manifestlerden gelecektir.

Temel güvenlik sınırı şudur: manifest yalnızca önceden tanımlı dosya işlemlerini çalıştırabilir. Shell, CMD, PowerShell, script veya indirilen executable çalıştırma desteği eklenmeyecektir. Bütün hedef yollar canonicalize edilerek seçilen oyun kökünün altında tutulacaktır.

## 2. Repository yapısı

```text
web/                 PHP uygulaması, admin paneli ve REST API
  app/               Controller, service, repository, security ve validation
  config/            Ortamdan okunan uygulama ayarları
  database/          MySQL migration ve demo seed dosyaları
  public/            Tek public web root ve statik assetler
  resources/         PHP view'ları
  routes/            Web ve API route tanımları
  scripts/           Güvenli ilk-admin ve bakım CLI araçları
  storage/           Public olmayan patch, log ve geçici dosyalar
loader/              Tek generic Tauri uygulaması
  src/               TypeScript UI, API istemcisi ve state katmanı
  src-tauri/src/     Rust patch, backup, download, detection ve security motoru
schemas/             Manifest JSON Schema sözleşmeleri
examples/            Güvenli örnek manifestler
docs/                Ek mimari ve test notları
```

## 3. Web mimarisi

Framework bağımlılığı olmadan cPanel uyumlu, küçük bir front-controller mimarisi kullanılacaktır. `public/index.php` tüm istekleri router'a aktarır. Controller'lar HTTP ayrıntılarını, service katmanı iş kurallarını, repository katmanı ise PDO prepared statement erişimini yönetir. HTML view'ları PHP template olarak tutulur. Patch ZIP'leri `public` dışında saklanır ve yalnızca kısa ömürlü, tek amaçlı indirme token'ı üzerinden stream edilir.

Mevcut yapı boş olduğu için korunacak eski kod bulunmamaktadır. Buna rağmen yeni katmanlar birbirinden ayrılarak ileride framework'e geçiş veya bağımsız API ölçeklendirmesi mümkün tutulacaktır.

## 4. Veritabanı mimarisi

Normalize MySQL şeması; kullanıcı/rol/abonelik, oyun/kategori/tespit, patch/sürüm/arşiv/action/release, loader config/sürüm, duyuru/banner, indirme ve audit alanlarına ayrılacaktır. Yayın durumu (`DRAFT`, `TESTING`, `PUBLISHED`, `DISABLED`) ile kanal (`stable`, `beta`, `internal`) birbirinden bağımsız tutulacaktır.

Manifest yayın anında immutable snapshot olarak saklanacaktır. Yeni düzenlemeler yeni patch sürümü üretir; mevcut yayın geçmişi korunur. Rollback, seçilen eski sürümü transaction içinde yeniden aktif kanal sürümü yapar ve audit kaydı oluşturur.

## 5. API mimarisi

API `/api/v1` altında sürümlenecek, istenen sürümsüz `/api/...` yolları uyumluluk amacıyla aynı handler'lara yönlendirilecektir. JSON hata zarfı, request ID, rate limit ve yetki kontrolü ortak middleware davranışıdır.

Başlıca endpointler:

- Kimlik: login, logout, current user.
- Katalog: oyun listesi, oyun detayı, uygun patch.
- Patch: manifest, kısa ömürlü download token ve tokenlı stream.
- Loader: remote branding config ve son loader sürümü.
- Admin: oyun/patch/category/user/config CRUD, kopyalama, toplu işlem, test, yayınlama ve rollback.

## 6. Admin ve Loader Oluşturucu

Admin paneli responsive koyu tema kullanır. Dashboard, oyunlar, kategoriler, patch sürümleri, görsel action builder, JSON manifest editörü, ZIP içerik ağacı, publish checklist, kullanıcılar, abonelikler, duyurular, bannerlar, loader ayarları/sürümleri, indirme kayıtları ve audit log ekranları modüler route/view'lar olarak uygulanacaktır.

Görsel builder yalnızca allow-list action tipleri üretir. JSON görünümü aynı modelin gelişmiş editörüdür ve kaydetme/yayınlama öncesi server-side şema ile güvenli yol doğrulamasından geçer. Loader'ın logo, renk ve bağlantıları remote config'tir; executable adı, publisher, application ID ve imzalama gibi alanlar build config'te kalır.

## 7. Manifest formatı

İlk format `schema_version: 1` olacaktır. Ana bölümler `game`, `detection`, `patch`, `archive`, `install_actions`, `integrity` ve `backup` olacaktır. Action sırası kararlıdır ve her action benzersiz ID taşır. Kaynak yollar archive köküne, hedef yollar oyun köküne göre relative olmak zorundadır.

Manifest JSON Schema hem PHP yayınlama doğrulamasına hem Rust deserialize/semantic validation katmanına referans olur. Loader bilinmeyen schema sürümünü reddeder; desteklenen eski sürümler açık migration fonksiyonlarıyla dönüştürülür.

## 8. Rust patch action sistemi

Rust tarafında `PatchActionHandler` trait'i ve tip bazlı handler registry bulunacaktır. Desteklenen v1 actionları: `COPY_FILE`, `COPY_DIRECTORY`, `REPLACE_FILE`, `DELETE_FILE`, `DELETE_DIRECTORY`, `CREATE_DIRECTORY`, `MOVE_FILE`, `RENAME_FILE`.

Kurulum motoru önce dry-run planı üretir, gerekli alan/backup boyutunu hesaplar, sonra transaction journal açar. Her destructive işlemden önce içerik hash'iyle backup alınır. Başarılı action journal'a fsync edildikten sonra ilerlenir. Hata durumunda actionlar ters sırayla rollback edilir. Kurulum sonunda installation manifest atomik olarak yazılır.

Uninstall; kurulum manifestindeki beklenen kurulu hash ile mevcut dosyayı karşılaştırır. Kullanıcı/oyun dosyası sonradan değişmişse işlem durdurulur ve açık conflict raporu döner; körlemesine overwrite yapılmaz.

## 9. Oyun tespiti

Steam discovery, Windows registry yerine Steam'in bilinen istemci yolları ve `libraryfolders.vdf`/`appmanifest_<id>.acf` dosyalarını okur. Her aday dizin manifestteki executable ve required-file kurallarıyla doğrulanır. Manuel seçim aynı doğrulayıcıdan geçer. Epic/manual kurallarının eklenmesi provider registry üzerinden yapılır; oyun adına özel branch yazılmaz.

## 10. Güvenlik modeli

- Password hashing PHP'nin güncel `PASSWORD_DEFAULT` algoritmasıyla yapılır.
- PDO emulated prepare kapalıdır; bütün sorgular parametrelenir.
- Admin formlarında CSRF, session rotation, secure/HttpOnly/SameSite cookie uygulanır.
- Upload MIME, uzantı, boyut ve ZIP yapısı doğrulanır; dosya adı rastgele üretilir.
- ZIP entry'lerinde absolute, UNC, drive-prefix, `..`, symlink/reparse ve normalize edilmiş kök dışı yollar reddedilir.
- Loader indirme sonrası SHA-256 ve boyutu doğrulamadan archive açmaz.
- Manifestte komut çalıştıran alan yoktur; bilinmeyen action fail-closed reddedilir.
- Download token hashlenmiş, kullanıcı/patch kapsamlı, kısa ömürlü ve tek kullanımlıdır.
- Secret'lar yalnız `.env` içinden gelir; repository'de örnek değerler bulunur.
- Admin değişiklikleri before/after JSON ile audit log'a yazılır.

## 11. Arayüz mimarisi

Loader UI, API/state/view ayrımıyla TypeScript bileşenlerinden oluşur. Katalog remote API'den gelir ve lokal cache çevrimdışı yalnız görüntüleme için kullanılır. Kurulum internet yokken başlamaz. Ana görünüm üst navigasyon, hero banner, filtrelenebilir cover grid, sağ duyuru/sidebar ve oyun detay drawer/page düzenini kullanır. Assetler özgün SVG/CSS placeholder'lardır.

## 12. Uygulama aşamaları

1. Repository, environment örnekleri, şema ve güvenlik yardımcıları.
2. MySQL migration/seed, PHP router, auth ve katalog API.
3. Admin CRUD, patch upload/builder, test/publish/rollback ve audit.
4. Tauri shell, dinamik katalog UI ve remote branding/cache.
5. Rust manifest validator, güvenli path/ZIP, download, detection, transaction, backup, rollback, uninstall ve verify.
6. Demo verileri, test fixture'ları, PHP/Rust/TypeScript doğrulamaları.
7. cPanel, Windows setup/build, admin ve patch oluşturma dokümantasyonu.

## 13. Doğrulama stratejisi

PHP syntax kontrolleri ve saf sınıflar için CLI testleri; SQL şema tutarlılık kontrolü; TypeScript type-check/build; Rust unit/integration testleri uygulanacaktır. Özellikle traversal/absolute/UNC path, ZIP Slip, hash mismatch, yarım transaction rollback, uninstall conflict ve bilinmeyen action negatif testleri bulunacaktır. Yerel dependency eksikliği kaynak üretimini durdurmaz; çalıştırılamayan kontroller sonuç raporunda açıkça belirtilir.

## 14. Teslim ölçütü

Tek loader API'den yayınlanan herhangi bir geçerli oyunu kod değişmeden gösterecek; admin kod yazmadan oyun ve patch sürümü oluşturup test/yayın yapabilecek; installer yalnız allow-list dosya işlemleriyle game root içinde transactional çalışacak; uninstall backup ve hash çatışma korumasıyla geri alabilecek; kurulum/build/cPanel/yeni patch akışları dokümante edilmiş olacaktır.
