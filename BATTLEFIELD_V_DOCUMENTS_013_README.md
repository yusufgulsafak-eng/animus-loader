# Animus Loader Battlefield V / Windows Belgeler onarimi

## Kaynak
Onceki GitHub 0.1.2 paketi ve orijinal Battlefield™ 5.zip uzerinden hazirlandi.

## Ne degisti?
- Loader 0.1.3: `dirs::document_dir()` ile Windows'un yonlendirilmis gercek Belgeler klasorunu bulur.
- Eski (1.0.0) Battlefield V ZIP'indeki hash'i sabit BAT kabul etmeye devam eder; bu eski BAT'i calistirmak yerine ayni sabit SHA-256 ile dogrulanmis JSON'u dogrudan gercek Belgeler klasorune kopyalar (yerellesmis dizin hatasini atlar).
- Yeni (1.0.1) ZIP icin `Install.bat` gercekten calistirilir; Loader `ANIMUS_DOCUMENTS_DIR` degerini guvenli Windows konumundan iletir. BAT hata kodu ve ciktilari raporlanir.
- Mevcut yedekleme, dosya hash'i, kurulumu dogrulama ve kaldirma yollarinda ortak Documents hedefi kullanilir.
- PHP validator, JSON Schema ve admin.js hem eski hem yeni **sabitlenmis** BAT hash'lerini dogrular; yeni hash icin manifest minimum surumu 0.1.3 zorunludur.
- GitHub projesinde uygulama surumleri 0.1.3 yapildi.

## Bilinen hash'ler
- Orijinal Install.bat: `83848a9cf76b60db8577e1978a0bab9c271a5a4a3a3a32a205614a7b69b0b9b8`
- Yeni Install.bat: `7016526b403eff2374a2874274a29bae97fff14daa10682d230f32716d2ff463`
- Yeni Battlefield V ZIP SHA-256: `cfb33da16e3893e8aea62c5c4599e79da9a3cfe4876e76181d01a1fea6639359`
- Yeni Battlefield V ZIP bytes: `20568583`
- JSON SHA-256 (degismedi): `f3c24860e661bedeb4d3b71e43c0cbf3b2f1fd40c3d13e64bf98420e45e41088`

## Uygulama
1. GitHub kaynak ZIP'ini var olan repo ile karsilastirip degistirilen kaynak dosyalarini commit et; `npm ci && npm test && npm run build` ve Windows GitHub Actions derlemesini calistir.
2. cPanel'deki `ManifestValidator.php`, `admin.js` ve ayni kaynak proje dosyalarini yedekleyip yeni surumlerle eslestir. `AdminService.php` degistirilmedi.
3. Windows uzerinde once eski aktif Stable 1.0.0 yamayi Loader 0.1.3 ile deneyebilirsin; yeni Loader eski BAT yerine ayni SHA-256'li JSON'u gercek Belgeler klasorune native kopyalar. Kurulumdan once gerekiyorsa daha once elle olusturulmus JSON'un yedegini al.
4. Yeni 1.0.1 yamasini kullanmak icin yeni ZIP'i (20 MB) **yeni bir HTTPS adrese** yukle, linki yeni SQL'de duzelt. Eski MediaFire adresi yeni ZIP'i sunmuyor. Bu SQL yalnizca DRAFT olusturur, aktif PUBLISHED 106'ya dokunmaz.
5. Windows'ta gercek kurulum, kaldirma, onceki JSON'u geri yukleme testleri gecince Admin panelden 1.0.1'i yayinla.
6. Kalici kaynak dosyalarini tek basina degistirmek veya eski SQL'i tekrar calistirmak yeterli DEGIL.

## Sinirlar
Bu oturumda Windows Tauri uygulamasi/kurulum gercek cihazda test EDILEMEDI. SQL hicbir sunucuya uygulanmadi. MediaFire'a yeni ZIP yuklenmedi. Gercek agdan indirme dogrulanmadi. Kaynak paketinin tamamindaki bagimsiz sorunlar cozulmus sayilmaz.
