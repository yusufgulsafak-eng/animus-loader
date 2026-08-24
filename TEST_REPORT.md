# Smoke Test Raporu

Tarih: 24 Ağustos 2026  
Kapsam: Tauri debug loader temel açılış/render testi ve yerel web/API bağımlılık kontrolü

## Test ortamı

- Proje: `C:\Users\yusuf\OneDrive\Masaüstü\test`
- Loader: `C:\Users\yusuf\OneDrive\Masaüstü\test\loader`
- Çalıştırma: `npm.cmd run tauri dev`
- Tauri process: `animus-patch-loader.exe` (PID 23216)
- Vite: `http://localhost:1420/`
- Loader API hedefi: `http://127.0.0.1:8080/api`
- Test sırasında gerçek PHP/MySQL/API servisi çalışmıyordu.

## Sonuç özeti

| Kontrol | Sonuç | Kanıt / gözlem |
|---|---|---|
| Debug loader başlıyor | BAŞARILI | Rust debug profili tamamlandı ve `animus-patch-loader.exe` çalıştı. |
| Giriş ekranı açılıyor | BAŞARILI | Gerçek Tauri penceresinde logo, e-posta/şifre alanları ve `Giriş Yap` butonu render edildi. |
| Ana pencere render oluyor | BAŞARILI | Geçici localhost test API'siyle girişten sonra header, hero, kullanıcı alanı, katalog ve sağ duyuru paneli görüntülendi. |
| Kütüphane açılıyor | BAŞARILI | Varsayılan authenticated görünümde `Kütüphane`, filtreler, arama ve oyun grid'i render edildi. |
| API kapalı hata yönetimi | BAŞARILI / İYİLEŞTİRİLEBİLİR | Giriş denemesi uygulamayı kapatmadı; form üzerinde `Failed to fetch` hata paneli gösterildi. Mesaj kontrollü fakat Türkçeleştirilmeli ve daha kullanıcı dostu olmalı. |
| Crash/panic kontrolü | BAŞARILI | Process test sonunda hâlâ çalışıyor ve `Responding=True`. Tauri terminalinde panic/hata çıktısı ve Windows Application logunda uygulama crash kaydı yok. |

## Test yöntemi

1. Çalışan Tauri penceresi process ve pencere başlığı üzerinden doğrulandı.
2. Giriş ekranı gerçek pencere ekran görüntüsüyle kontrol edildi.
3. Port 8080 kapalıyken test giriş isteği gönderildi ve hata görünümü doğrulandı.
4. Gerçek proje verisi değiştirilmeden, yalnız test süresince bellekte çalışan localhost mock API kullanıldı.
5. Mock API; login, loader config ve oyun listesi cevapları döndürdü. Ana shell ve kütüphane render edildikten sonra mock süreç durduruldu ve port 8080 kapatıldı.
6. Loader process yanıt durumu, terminal çıktısı ve Windows Application event log kontrol edildi.

## Bulgular

- Authenticated görünüm doğrudan kütüphane içeriğini gösteriyor; üst menüdeki `Kütüphane` düğmesi için ayrı bir click/route handler bulunmuyor. Şu an kütüphane varsayılan ana içerik olduğundan görünüm açılıyor, ancak ileride ayrı sayfalar eklenecekse navigation state/route uygulanmalı.
- API kapalı hata yakalanıyor ancak tarayıcı kaynaklı `Failed to fetch` metni doğrudan kullanıcıya gösteriliyor. Önerilen kullanıcı mesajı: `Sunucuya ulaşılamıyor. Lütfen bağlantınızı kontrol edip tekrar deneyin.`
- Loader kapatılmadı; debug process ve Vite geliştirme sunucusu çalışır durumda bırakıldı.
- Smoke test için kullanılan geçici API tamamen durduruldu; port 8080 dinlenmiyor.

## PHP/MySQL bağımlılık kontrolü

| Bileşen | Durum |
|---|---|
| PHP | Kurulu değil / PATH üzerinde bulunamadı |
| MySQL client | Kurulu değil / PATH üzerinde bulunamadı |
| MySQL server | Çalışmıyor ve port 3306 dinlenmiyor |
| MariaDB | Kurulu değil |
| Composer | Kurulu değil; mevcut proje Composer paketi kullanmadığı için ilk çalıştırma açısından zorunlu görünmüyor |

Winget üzerinde doğrulanan paketler:

- `PHP.PHP.8.4` (PHP 8.4)
- `Oracle.MySQL` (MySQL 8.4)
- Alternatif: `MariaDB.Server`

## Yerel web/API için önemli not

`web/public/.htaccess` Apache/cPanel rewrite kurallarını içeriyor. PHP built-in server `.htaccess` çalıştırmaz. README içindeki yalnız `php -S 127.0.0.1:8080 -t web/public` komutu `/api/...` ve `/admin/...` gibi dinamik yolları tek başına front controller'a yönlendirmez. Yerel geliştirmede uygun bir PHP router dosyası/komutu veya Apache kullanılmalıdır.

Ayrıca `.env.example` içinde `CORS_ORIGINS` tanımlı olmasına rağmen mevcut PHP kodunda bu değeri okuyup `Access-Control-Allow-*` başlıklarını ve `OPTIONS` preflight cevabını üreten bir middleware bulunamadı. Tauri dev origin'i (`http://localhost:1420`) ile gerçek API arasında end-to-end bağlantı kurulmadan önce CORS katmanı tamamlanmalıdır.
