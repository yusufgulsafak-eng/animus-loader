# Küçük FC5 paketi için native arşiv işlemi

Bu değişiklik `APPEND_FAT_DAT` işlemini ekler. ZIP yalnız `ceviri.bin` içerir;
oyuncunun mevcut DAT dosyası yeniden indirilmez. İşlem FAT2 v10 arşivini ve
base SHA-256 değerlerini kontrol eder; yeni kaynağı DAT sonuna 8 bayt hizalayarak
ekler ve tek FAT kaydını günceller. Diğer FAT kayıtları ve eski DAT baytları
korunur. Başka oyun sürümü veya bozuk mini arşiv kabul edilmez.

## Kurulum ve geri alma

- Bu işlem manifestte tek başına kullanılır. `REPLACE_FILE patch.dat/patch.fat`
  eylemleriyle birlikte yayınlanmaz.
- Minimum Loader sürümü `0.1.1`, process_name `FarCry5.exe` olmalıdır.
- FAT yedeği, payload ve eski DAT uzunluğu değişiklikten önce kaydedilir.
  Kurulum kaydı da ilk oyun dosyası yazımından önce kalıcı yazılır.
- Yarım append veya FAT değişimi sonrasında Yamayı Kaldır güvenli olarak
  eski FAT'i geri koyup DAT'i eski uzunluğa döndürür. Önce base bölge ve
  eklenen baytlar kontrol edilir. Kullanıcının farklı verisi varsa işlem durur.
- Geri alma oyunun çalışmadığını kontrol eder. Windows DAT dosyası işlem
  boyunca özel erişimle açılır.
- Doğrulama hem DAT hem FAT hash'ini kontrol eder.
- Büyük DAT yedeği tutulmaz: geri alma için yalnız FAT, payload ve kayıt gerekir.

## Sunucu ve Loader birlikte güncellenir

1. Kaynak değişikliklerini uygulayın ve test/build workflow sonucunu kontrol edin.
2. Yeni Loader 0.1.1 kurulumunu deneyin; eski Loader yeni action'ı çalıştıramaz.
3. `web/app/Services/ManifestValidator.php`, `AdminService.php` ve
   `web/public/assets/admin.js` dosyalarını cPanel'de güncelleyin.
4. Veritabanı yedeğinizden sonra `web/database/009_fat_dat_action.sql` çalıştırın.
   Bu yalnız enum'a yeni eylem ekler; hiçbir oyunu yayınlamaz.
5. Yeni bir DRAFT/internal patch sürümü oluşturun. `examples/fc5-native-action.json`
   alanlarını tek action olarak kullanın. Mağazalar steam ve ubisoft; oyun kökü
   altındaki hedef `data_final/pc/patch.dat` ve `.fat` çiftidir.
6. İndirilecek ZIP'in gerçek SHA-256 ve bayt boyutunu arşiv kaydına yazın.
   MediaFire adresi farklı olduğunda URL de güncellenmelidir.
7. Ayrı oyun kopyasında kurulum, doğrulama, oyunda menü ve kaldırma testleri
   geçtikten sonra yayınlayın. Menü görünümü yalnız dosya testleriyle garanti edilmez.

Eski mini REPLACE_FILE kurulumu oyunun büyük arşivini değiştirdiyse önce
Loader'ın mevcut yedeğinden kaldırma işlemiyle orijinali geri getirin. Base
hash uyuşmazlığını atlamayın; bu değişiklik eksik oyun verisini yeniden üretmez.

## Testler

`.github/workflows/fc5-native-archive.yml` ilgili dalda Linux native testleri,
PHP doğrulama testleri ve Windows test/build işlemlerini çalıştırır. Native
testler gerçek uygulama modülünü küçük sentetik FAT/DAT dosyalarıyla kullanır.
Kapsam: payload yeniden çıkarımı, diğer kayıtların korunması, yanlış base/hash,
bozuk FAT, güvensiz yollar, yarıda kesilme, FAT-only değişikliği, bozuk yedek,
geri alma sırasında kullanıcı verisinin korunması ve kayıt yazma hatası.

`fat-dat-check` yalnız QA için hazırlanmış komut satırı test aracıdır. Yama ZIP'ine
konmaz ve Loader tarafından başlatılmaz.


