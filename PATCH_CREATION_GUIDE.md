# Türkçe Yama Oluşturma Rehberi

Bu rehber, loader kaynak kodunu değiştirmeden yeni oyun ve yama yayınlama akışını anlatır.

## A. Yeni oyun oluşturma

1. /admin adresinde giriş yapın.
2. Oyunlar bölümünde Yeni Oyun Ekle seçeneğini açın.
3. Benzersiz oyun adı ve URL uyumlu slug girin.
4. Cover için 2:3, banner için yaklaşık 8:3 oranlı size ait görseller kullanın.
5. Steam oyunuysa sayısal Steam App ID girin.
6. Ana executable alanını oyun köküne göre relative yazın: ExampleGame.exe.
7. Process name alanına Windows Task Manager'da görünen executable adını girin.
8. Required files alanına yanlış klasör seçimini kesin olarak engelleyen dosyaları her satıra bir tane yazın.
9. Optional files yalnız ek güven/versiyon sinyali içindir.
10. Ücretsiz/Premium, çeviri yüzdesi, mağazalar ve minimum loader sürümünü seçin.
11. İlk test bitene kadar oyunu pasif bırakın.

Yollar absolute olamaz. Drive harfi, UNC, başta slash, nokta segmenti veya üst dizine çıkış reddedilir.

## B. Benzer oyunu kopyalama

Oyun listesindeki Kopyala düğmesi aynı ayarlarla pasif bir kayıt üretir. Yeni kayıt adının sonunda Kopya bulunur ve slug çakışmayacak biçimde değiştirilir. Yeni oyunun App ID, executable, görseller ve required files alanlarını mutlaka güncelleyin.

## C. Patch ZIP hazırlama

Arşivin içinde yalnız kurulacak veri bulunmalıdır:

~~~text
example-game-tr-1.0.0.zip
  files/
    Game.locres
    Turkish/
      localization.dat
~~~

ZIP içine installer EXE, BAT, CMD, PowerShell veya çalıştırılabilir script koymayın. Loader bunları çalıştırmaz. Kaynak action yolları ZIP köküne göre relative yazılır.

Arşiv kuralları:

- Entry adlarında ../, absolute path, drive prefix veya UNC bulunamaz.
- Symlink/reparse entry kabul edilmez.
- Her dosyanın açılmış boyutu ve toplam dosya sayısı sınırlandırılır.
- Şifreli veya bozuk ZIP yayınlanmamalıdır.
- Aynı hedefe birden fazla action yazmaktan kaçının.

## D. Yeni patch sürümü

1. Yamalar veya Patch Builder bölümünü açın.
2. Oyunu seçin.
3. SemVer biçiminde patch sürümü girin: 2.3.0.
4. Desteklenen oyun sürümünü yazın.
5. Değişiklik listesini anlaşılır girin.
6. stable, beta veya internal kanalını seçin.
7. İlk deneme için internal ve DRAFT kullanın.
8. Minimum loader version alanını gerekli patch engine sürümüne göre belirleyin.
9. ZIP'i seçip yükleyin.

Sunucu rastgele storage adı üretir, SHA-256 ve byte boyutunu kendisi hesaplar, ZIP ağacını doğrular ve arşivi public alanın dışında saklar.

## E. Görsel action builder

Draft oluşturulduktan sonra verilen Patch Version ID'yi builder alanında kullanın.

### COPY_FILE

ZIP'teki bir dosyayı yeni hedefe kopyalar. Hedef zaten varsa otomatik backup alınır.

~~~text
source: files/Game.locres
destination: Content/Localization/Game.locres
backup: true
~~~

### COPY_DIRECTORY

Kaynak directory ağacını hedefin altına kopyalar. Her mevcut hedef dosyası ayrı ayrı backup ve journal kaydı alır.

### REPLACE_FILE

Hedef dosyanın mevcut olması beklenen açık niyetli değiştirme actionıdır. Mevcut içerik işlem öncesi SHA-256 ile yedeklenir.

### DELETE_FILE ve DELETE_DIRECTORY

Silmeden önce mevcut dosyaları otomatik yedekler. Directory silmede içerideki her dosya installation manifest'e kaydedilir.

### CREATE_DIRECTORY

Yalnız game root altında directory oluşturur. Uninstall sırasında yalnız boşsa kaldırılır.

### MOVE_FILE ve RENAME_FILE

Source ve destination oyun köküne göre relative'tir. Mevcut destination varsa o da backup alınır. Uninstall işlemi taşıma yönünü ters çevirir.

## F. Backup seçimi

Bir action mevcut dosyayı değiştiriyor veya siliyorsa admin seçimi false olsa bile güvenli motor destructive işlem öncesi backup davranışını korur. Aktif kurulumun zorunlu backup'ı Backup Manager'dan silinemez.

## G. Manifest testi ve dry run

Actionları kaydedin ve Manifest Test düğmesine basın. Şunlar hatasız olmalıdır:

- schema_version destekleniyor.
- Oyun ve patch ID'leri mevcut.
- Arşiv SHA-256 ve boyutu mevcut.
- En az bir izinli action var.
- Kaynak/hedef yollar güvenli.
- Gerekli source alanları dolu.
- Detection required files güvenli.

Test loader hesabıyla Dry Run çalıştırın. Rapor; oluşturulacak, değiştirilecek, silinecek ve yedeklenecek dosya sayılarını; indirme ve tahmini disk kullanımını gösterir. Dry Run dosya değiştirmez.

## H. Test kanalı

1. Sürümü internal kanalında bırakın.
2. Admin/internal kullanıcıyla loader'da oyunu görün.
3. Sahte veya ayrı test oyun kökünde kurulum yapın.
4. Kurulum sonrası Dosyaları Doğrula çalıştırın.
5. YAMAYI KALDIR ile orijinal hash ve dosyaların geri geldiğini doğrulayın.
6. Oyun çalışırken loader'ın işlemi reddettiğini test edin.
7. Bir kurulu dosyayı elle değiştirip uninstall conflict korumasını test edin.

Gerçek kullanıcı verisi üzerinde doğrudan ilk test yapmayın.

## I. Yayınlama

Publish öncesi checklist bütün maddeleri geçmelidir. Yayın işlemi seçilen kanalın önceki aktif sürümünü ARCHIVED yapar, manifest snapshot'ını immutable kaydeder ve release channel pointer'ını transaction içinde yeni sürüme taşır.

stable normal kullanıcıya, beta test kullanıcılarına, internal yalnız yetkili iç kullanıcılara yöneliktir.

## J. Yeni sürüm ve güncelleme

Eski patch kaydını değiştirmek yerine Yeni Yama Sürümü oluşturun. Sürüm numarasını yükseltin, yeni ZIP/action listesini test edin ve aynı kanalda yayınlayın. Loader API'deki yeni version ile lokal installation version'ı karşılaştırıp Güncelleme Var durumunu gösterir.

Güncelleme kurulumu da normal transaction kurallarını izler. Production'da eski sürümü kaldırıp yeni sürümü kuran kontrollü update orkestrasyonu kullanılmalıdır; kullanıcı dosyasında conflict varsa işlem durdurulur.

## K. Sorunlu yayını geri alma

Patch History içinden daha önce doğrulanmış sürümü seçip rollback release işlemi uygulayın. Bu yalnız kanalın aktif sürüm işaretçisini eski sürüme taşır ve audit log yazar. Kullanıcı makinesindeki dosyalar ancak kullanıcının Güncelle/Kur akışıyla değiştirilir.

## L. Export/import

Oyun export dosyası; oyun alanları, kategoriler, detection kuralları, patch metadata ve action yapılarını içermeli; API token, kullanıcı verisi, storage server path, APP_KEY veya download token içermemelidir. Import sonrası kayıt DRAFT/Pasif açılmalı ve yeniden doğrulanmadan yayınlanmamalıdır.

