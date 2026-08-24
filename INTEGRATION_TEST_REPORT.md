# Entegrasyon Test Raporu

Tarih: 24 Ağustos 2026  
Proje: `C:\Users\yusuf\OneDrive\Masaüstü\test`

## Sonuç özeti

- PHP/MySQL API ve admin paneli `http://127.0.0.1:8080` üzerinde gerçek veritabanıyla çalışıyor.
- Production kataloğunda tam 38 aktif oyun var; bu oyunlara sahte patch bağlanmamış durumda.
- Loader TypeScript testleri 6/6 geçti ve Vite production build başarıyla tamamlandı.
- Önceden üretilmiş debug Tauri EXE güncel Vite arayüzüyle açıldı; süreç çalışır ve `Responding=True` durumunda kaldı.
- Güncel Rust test derlemesi Windows Uygulama Denetimi tarafından `futures_macro-*.dll` dosyasında `os error 4551` ile engellendi. Güvenlik ayarı değiştirilmedi.
- Bu nedenle güncel Rust testleri ve release installer bu çalışmada PASS sayılmadı.

## Ana entegrasyon kontrol listesi

| # | Kontrol | Sonuç | Kanıt / not |
|---|---|---|---|
| 1 | Migration | PASS | 001-004 atlandı, 005 branding ve 006 icon migration uygulandı. |
| 2 | Güvenli admin oluşturma | PASS | Sabit production şifresi olmayan CLI akışı ve yerel test admini çalışıyor. |
| 3 | Admin login | PASS | Session/cookie ile `/admin` 200; CSRF token üretildi. |
| 4 | 38 oyun | PASS | MySQL ve API katalog sayısı 38. |
| 5 | `/api/games` | PASS | Gerçek session ile `meta.count=38`. |
| 6 | Loader login | PASS (önceki smoke) | Gerçek API login ve kullanıcı/abonelik cevabı doğrulandı. |
| 7 | Loader katalog | PASS (önceki smoke) | Loader logunda API üzerinden 38 oyun yüklendi. |
| 8 | Arama | PASS | `catalog.test.ts`. |
| 9 | Filtreler | PASS | all/installed/update/free/premium birim testleri. |
| 10 | Oyun detay ekranı | PASS (smoke) | API detay sözleşmesi ve loader modal akışı doğrulandı. |
| 11 | Test patch download | PASS (önceki entegrasyon) | Tek kullanımlık download token ile fixture ZIP indirildi. |
| 12 | SHA-256 | PASS (önceki entegrasyon) | İndirilen fixture hash'i manifest ile eşleşti. |
| 13 | Backup | PASS (önceki Rust testi) | Değiştirilen dummy dosya yedeklendi. |
| 14 | Install | PASS (önceki Rust testi) | Yalnız geçici Patch Engine Test Game dizini kullanıldı. |
| 15 | Verify | PASS (önceki Rust testi) | Kurulum çıktıları doğrulandı. |
| 16 | Uninstall | PASS (önceki Rust testi) | Oluşturulan dummy dosya kaldırıldı. |
| 17 | Restore | PASS (önceki Rust testi) | `original.txt` özgün içeriğine döndü. |
| 18 | CORS | PASS | İzinli `http://localhost:1420` için 204 ve exact origin; kötü origin yansıtılmadı. |
| 19 | Premium authorization | PASS (önceki entegrasyon) | Yetkisiz premium download token isteği 403. |
| 20 | Crash/panic | KISMİ PASS | Mevcut debug süreç yanıt veriyor; güncel Rust rebuild Windows policy nedeniyle tamamlanamadı. |

## Branding medya kabul testleri

| Senaryo | Sonuç |
|---|---|
| JPG upload | PASS — 200, `image/jpeg` media endpoint |
| PNG upload | PASS — 200 |
| WebP upload | PASS — 200 |
| Gerçek MP4 upload | PASS — 200 |
| MP4 byte range | PASS — `206 Partial Content`, `Accept-Ranges: bytes` |
| PHP içeriğini `.jpg` yapma | PASS — 422 ile reddedildi |
| EXE upload | PASS — 422 ile reddedildi |
| Limit üstü video | PASS — 422 ile reddedildi |
| Bilinmeyen background type | PASS — loader birim testinde default fallback |
| Eksik video URL + fallback | PASS — loader birim testinde image fallback |
| Login video → library video cleanup | PASS — eski video `pause`, `src` kaldırma ve `load` çağrıları test edildi |
| Login=video / Library=image | PASS — bağımsız remote config cevabı |
| Login=image / Library=video | PASS — bağımsız remote config cevabı |
| Varsayılana dön | PASS — iki slot default değerlerine döndü ve aktif olmayan dosyalar silindi |
| API geriye uyumluluk | PASS — eski top-level alanlar korunurken `branding` eklendi |
| Tam görsel autoplay/codec testi | MANUEL DOĞRULAMA GEREKLİ — WebView2 codec/görsel oynatma otomatik UI aracıyla gözlenmedi |

## Admin ve medya sonuçları

- Admin view aktarımındaki `EXTR_SKIP` isim çakışması düzeltildi; panel artık gerçek oyun/config verilerini render ediyor.
- Admin HTML'de 38 oyun ve iki branding medya formu mevcut.
- Cover, banner ve icon upload/delete gerçek endpoint ile çalışıyor. Icon WebP testi sonrası kayıt ve dosya güvenli biçimde temizlenip placeholder'a döndü.
- Patch ZIP/SHA-256/action/manifest/publish akışları korunuyor. Arşivlenmiş sürümler için açık Rollback işlemi eklendi; fixture sürümünde admin action HTTP 200 ve PUBLISHED sonucu doğrulandı.
- Branding storage patch arşivlerinden ayrı ve public web root dışında: `web/storage/media/branding`.

## Engeller

Güncel `cargo test` şu işletim sistemi politikası hatasıyla derleme aşamasında durdu:

```text
LoadLibraryExW failed: Uygulama Denetimi ilkesi bu dosyayı engelledi. (os error 4551)
```

Bu bir Rust kaynak derleme hatası olarak değerlendirilmedi; Windows güvenlik ayarı değiştirilmedi. Entegrasyon testleri tamamen yeşil olmadığı için release installer üretilmedi.
