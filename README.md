# Animus Türkçe Yama Platformu

Tek bir generic Windows loader ile sınırsız oyunun Türkçe yamasını yöneten; PHP/MySQL web sitesi, REST API, admin/loader oluşturucu ve Tauri/Rust istemcisinden oluşan platform.

Oyun adı veya kurulum davranışı loader kaynak kodunda hardcode edilmez. Admin panelinin yayınladığı schema v1 manifestleri katalog, tespit, patch arşivi ve izinli dosya actionlarını tanımlar.

## Bileşenler

- web/: cPanel uyumlu PHP 8+ API, web sitesi ve admin paneli.
- loader/: Tauri 2, Rust ve TypeScript tek Windows uygulaması.
- schemas/: paylaşılan manifest JSON Schema.
- examples/: telifsiz demo manifest, patch içeriği ve sahte oyun kökü.
- tests/: bağımlılıksız PHP güvenlik testleri.

## Hızlı başlangıç

1. .env.example dosyasını .env olarak kopyalayın ve gerçek veritabanı bilgilerini girin.
2. MySQL veritabanı oluşturup şu komutu çalıştırın:

~~~powershell
php web/scripts/migrate.php
php web/scripts/create_admin.php
php -S 127.0.0.1:8080 -t web/public
~~~

3. Admin panelini http://127.0.0.1:8080/admin adresinden açın.
4. Loader için loader/.env.example dosyasını loader/.env adıyla kopyalayın.

~~~powershell
cd loader
npm install
npm run tauri dev
~~~

Production kurulum için CPANEL_INSTALL.md, Windows araçları için SETUP_WINDOWS.md ve loader derleme için LOADER_BUILD.md belgelerini izleyin.

## Güvenlik sınırı

Manifest CMD, PowerShell, shell, script veya executable çalıştıramaz. Bilinmeyen action reddedilir. ZIP ve tüm relative yollar iki tarafta doğrulanır. Değiştirilen/silinen dosyalar işlem öncesi hashli backup'a alınır; journal her action sonrasında atomik yazılır.

## Test

~~~powershell
php tests/php/run.php
cd loader
npm test
cargo test --manifest-path src-tauri/Cargo.toml
~~~

MySQL integration testleri için ayrı test veritabanı ve .env gerekir.

## Dinamik branding medyası

Admin panelindeki **Loader Oluşturucu → Arka Plan ve Medya** bölümünden login ve ana loader arka planları bağımsız olarak varsayılan, resim veya video yapılabilir. Ayarlar `/api/loader/config` üzerinden gelir; loader EXE yeniden derlenmez. Medya dosyaları private branding storage alanında tutulur ve content-addressed URL değişimiyle cache yenilenir.

Resim allow-list'i JPG/JPEG, PNG ve WebP; video allow-list'i MP4 ve WebM'dir. Video sessiz, loop ve autoplay çalışır. Oynatma hatasında fallback resim, ardından mevcut Animus/CSS arka planı kullanılır.
