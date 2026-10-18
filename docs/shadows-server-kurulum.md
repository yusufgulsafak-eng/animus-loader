# Shadows sunucu kurulumu

Bu yapılandırma, gönderilen ORIJINAL `ANIMUS_AC_SHADOWS.zip` içindir.
Loader **0.1.5** gerekir. Gerçek oyunda kurulum/kaldırma testi henüz yapılmadı.

| Alan | Değer |
| --- | --- |
| MediaFire | https://www.mediafire.com/file/pi789nzgd1gyle1/ANIMUS_AC_SHADOWS.zip/file |
| Dosya adı | `ANIMUS_AC_SHADOWS.zip` |
| Boyut | `83350413` byte |
| ZIP SHA-256 | `931fd60132db50320a7db4bc815a371d752352a7acdb4e57ee1153d65899aed0` |
| Taslak sürüm | `1.0.0-shadows.1` |
| Kanal / durum | `internal` / `DRAFT` |
| Kurulum işlemi | Tek `INSTALL_SHADOWS`; yedekleme açık |

Gönderilen raporun hash'i, boyutu ve 1317 dosyasının yolları/boyutları hazırlanan
yerel ZIP ile eşleşti. MediaFire sayfasına bu çalışma ortamından erişilemedi;
uzaktaki dosyanın baytları ayrıca doğrulanmadı. Loader indirdiği ZIP'in boyutunu ve
SHA-256 değerini kurulumdan önce kontrol eder.

## Uygulama sırası

1. Sunucuda aşağıdaki üç dosyayı aynı göreli konumlarına yükle. Projedeki `web/`
   klasörü cPanel uygulama köküne karşılık gelir; sunucundaki dizilimi esas al.
   Güncellenen dosyalar: `app/Services/ManifestValidator.php`,
   `app/Services/AdminService.php`, `public/assets/admin.js`.
2. phpMyAdmin'de Animus uygulamasının veritabanını seç. Harici arşiv alanları
   henüz yoksa `007_external_patch_sources.sql` dosyasını uygula. Ardından
   `030_shadows_loader_action.sql` dosyasını içe aktar.
3. `031_shadows_mediafire_draft.sql` dosyasını içe aktar. Çıktıda gerçek
   `game_id`, `patch_id` ve `staged_version_id` gösterilir. Bunları not al.
4. Yönetim panelinde bu sürümü açıp manifest doğrulamasını çalıştır. Sonuçta
   yalnızca bir `INSTALL_SHADOWS` işlemi, beş gerekli FORGE ve minimum Loader
   `0.1.5` bulunmalıdır. Arşiv hash'ini işlemdeki manifest hash'iyle değiştirme;
   bunlar iki farklı dosyanın hash'leridir.
5. Loader üzerinden test için sürümü yönetim panelinden **internal kanalında
   yayınla**, test hesabının release kanalını `internal` yap ve oyun pasifse
   oyun kaydını aktif et. Yeni oyun aktif edilene kadar Loader listesinde
   görünmez. Aktif oyun kartı genel katalogda da görünür; yama internal kanalında
   kalır. Test bilgisayarına Loader 0.1.5 kur, oyunun ana klasörünü seç ve Yama
   Kur işlemini çalıştır.
6. Kurulumdan sonra oyunda Türkçe metinleri ve Loader'dan kaldırmayı test et.
   Kurulum/kaldırma veya yarım kalan işlemde sorun çıkarsa Loader logunu ve hata
   mesajını sakla. Test sonucu olmadan stable yayına geçme.

SQL mevcut ORIJINAL kaydını slug üzerinden bulur. Yoksa **pasif**, premium bir
oyun ve internal taslak oluşturur; yeni oyunun erişim türünü panelden
değiştirebilirsin. Mevcut oyunun erişim türü, aktiflik durumu ve görselleri
korunur. Mevcut daha yüksek minimum Loader sürümü düşürülmez.

İki uygun oyun, birden çok yama veya aynı taslak sürüm varsa SQL hata vererek
işlemi geri alır. Yayınlanmış bir Shadows sürümünün ortak algılama profili
farklıysa da değişiklik yapmaz. Hata alındığında kimlikleri tahmin ederek SQL'i
tekrar yazma; hata mesajını kontrol et. Başarılı içe aktarma sonrasında aynı
taslağı tekrar oluşturmak gerekmez.

SQL yayın kanallarını ve genel Loader alt sürümünü güncellemez; başka oyunların
yayınlarını değiştirmez. Yayın yönetim panelinin doğrulama ve audit akışından
yapılır. Bu dosyaların hazırlanması canlı sunucuya uygulandıkları anlamına gelmez.

Alanların JSON karşılığı `examples/shadows-server-profile.json` dosyasındadır.
Bu bir sunucu ayar özetidir; gerçek veritabanı ID'leri olmadığı için doğrudan
Loader manifesti olarak kullanılmaz.

## Yapılan kontroller

MariaDB 10.11.14 üzerinde 13 kontrol geçti: yeni ve mevcut oyun, daha yüksek
Loader alt sürümünün korunması, eksik migration, yinelenen taslak, belirsiz
oyun/yama, yayınlanmış profilin korunması, son işlemde hata sonrası tam geri
alma ve enum migration'ının tekrar uygulanması. SQL'den oluşan gerçek manifest
`ManifestService` ile üretildi ve PHP doğrulayıcısından geçti. Mevcut 41 PHP
güvenlik/manifest kontrolü de tekrar geçti. Bunlar gerçek oyunda test değildir.
