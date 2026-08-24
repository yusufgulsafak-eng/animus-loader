# cPanel Kurulum Rehberi

## 1. Gereksinimler

- PHP 8.2+ ve PDO MySQL, mbstring, fileinfo, zip, openssl.
- MySQL/MariaDB ve InnoDB/utf8mb4.
- Apache mod_rewrite.
- HTTPS sertifikası.
- Patch boyutuna uygun disk ve inode kotası.

## 2. Dosya yerleşimi

En güvenli yerleşim:

~~~text
/home/account/turkce-yama/web/          uygulama
/home/account/turkce-yama/storage/      public olmayan patch alanı
/home/account/public_html/              web/public içeriği veya document root
~~~

Mümkünse domain/subdomain Document Root'unu doğrudan web/public klasörüne ayarlayın. web/app, database, scripts ve storage web üzerinden erişilebilir olmamalıdır.

## 3. Veritabanı

1. cPanel MySQL Databases ekranında veritabanı ve ayrı bir kullanıcı oluşturun.
2. Kullanıcıya yalnız bu veritabanı için ALL PRIVILEGES verin.
3. phpMyAdmin ile sırasıyla web/database/001_schema.sql ve 002_demo_seed.sql dosyalarını import edin.
4. Demo veriler istenmiyorsa yalnız 001_schema.sql kullanın.

## 4. Environment

Repository .env.example dosyasını .env adıyla kopyalayın. DB_HOST cPanel sağlayıcısına göre localhost olabilir. APP_URL HTTPS domain olmalıdır. APP_DEBUG production'da false kalmalıdır.

PATCH_STORAGE_PATH için public_html dışındaki mutlak dizini kullanın:

~~~text
PATCH_STORAGE_PATH=/home/account/turkce-yama/storage/patches
~~~

APP_KEY güçlü rastgele değer olmalı; örnek veya varsayılan değer bırakmayın.

## 5. Dosya izinleri

- PHP kaynakları: 0644
- Klasörler: 0755
- Patch storage/log/tmp: web server kullanıcısına yazılabilir 0750
- .env: mümkünse 0600

0777 kullanmayın. Patch klasörünü public_html altında bırakmayın.

## 6. PHP limitleri

Patch boyutuna göre MultiPHP INI Editor içinde upload_max_filesize, post_max_size, max_execution_time ve memory_limit değerlerini ayarlayın. post_max_size, upload_max_filesize değerinden büyük olmalıdır. Büyük arşivler için hosting limitlerini sağlayıcıyla doğrulayın.

## 7. İlk admin

SSH/Terminal erişimi varsa:

~~~bash
php web/scripts/create_admin.php
~~~

Araç sabit şifre üretmez; güçlü şifreyi interaktif ister. Terminal yoksa komutu yerel güvenli ortamda aynı production veritabanına karşı çalıştırın veya hosting sağlayıcısından tek seferlik terminal isteyin. phpMyAdmin'e düz şifre yazmayın.

## 8. HTTPS ve cron

AutoSSL'i etkinleştirin ve HTTP'yi HTTPS'e yönlendirin. Süresi dolan api_tokens, download_tokens ve rate_limits kayıtlarını günlük cron ile temizleyen bakım komutu eklenebilir; tablolar istek sırasında da süresi geçmiş rate-limit satırlarını temizler.

## 9. Yayın kontrolü

- /health JSON dönmeli.
- /api/loader/config 200 dönmeli.
- /admin giriş sayfası açılmalı.
- storage URL ile doğrudan indirilememeli.
- .env ve SQL dosyaları web üzerinden erişilememeli.
- HTTPS sertifika zinciri loader makinesinde geçerli olmalı.

## 10. Yüklenecek dosyalar

cPanel'e web klasörünün app, database, public, resources, routes, scripts ve storage yapısını; repository .env dosyasını ve schemas klasörünü yükleyin. loader kaynak kodunu web sunucusuna yüklemek zorunlu değildir. Node modules, Rust target ve geliştirme cache klasörlerini yüklemeyin.

## 11. Branding medya limitleri

`.env` içinde `BRANDING_MEDIA_STORAGE_PATH`, `MAX_IMAGE_UPLOAD_SIZE` ve `MAX_VIDEO_UPLOAD_SIZE` değerlerini hosting limitleriyle uyumlu ayarlayın. `web/storage/media/branding` web root dışında kalmalı ve PHP kullanıcısı tarafından yazılabilir olmalıdır. `post_max_size` ile `upload_max_filesize`, seçtiğiniz video limitinden büyük olmalıdır. Medyayı storage yolundan doğrudan yayınlamayın; `/media/branding/{random-name}` endpointini kullanın.
