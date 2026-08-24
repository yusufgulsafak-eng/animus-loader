# Admin Paneli Kullanım Rehberi

## Giriş ve roller

İlk super admin scripts/create_admin.php ile oluşturulur. Normal kullanıcı, tester, admin ve super_admin rolleri bulunur. stable/beta/internal release channel rol ve kullanıcı seçimiyle sınırlandırılır.

## Dashboard

Toplam/aktif oyun, patch, stable/beta yayın, kullanıcı ve indirme sayaçlarını gösterir.

## Oyunlar

Arama ad, slug ve Steam App ID üzerinde çalışır. Yeni Oyun formu katalog, mağaza, tespit ve erişim bilgilerini toplar. Kopyala benzer yapıdaki oyun için pasif kopya oluşturur.

## Patch Builder

ZIP yükleme server-side MIME/uzantı/boyut/ZIP entry doğrulamasından geçer. SHA-256 ve byte boyutu tarayıcıya güvenilmeden hesaplanır. Action builder yalnız allow-list türleri üretir.

## Manifest Editor ve Patch Test

Version ID ile server'ın ürettiği gerçek manifest görüntülenir. Yayın kontrolü schema, archive, action ve yol güvenliğini tekrar çalıştırır. Hata varsa PUBLISHED durumuna geçiş yapılmaz.

## Loader Oluşturucu

Görünen uygulama adı, kütüphane başlığı, lime/accent rengi, logo/banner/login arka planı ve sosyal/destek URL'leri remote config olarak saklanır.

Executable adı, publisher, application ID ve signing certificate remote ayar değildir. Bunları loader build config'inde yönetin.

## Audit

Oyun oluşturma/düzenleme/kopyalama, action düzenleme, patch yayınlama ve loader config değişiklikleri actor, entity, before/after JSON, IP hash ve zamanla kaydedilir. Audit kayıtlarını normal CRUD ile silmeyin.

## Operasyon önerileri

- Her patch'i önce internal test edin.
- Stable yayın öncesi kur/install/verify/uninstall/restore döngüsünü tamamlayın.
- Eski archive'ı aynı storage adıyla değiştirmeyin.
- Admin hesabını paylaşmayın; her yöneticiye ayrı hesap verin.
- Production'da APP_DEBUG=false ve HTTPS zorunlu tutun.

## Arka Plan ve Medya

1. **Loader Oluşturucu** sayfasını açın.
2. Login veya Loader Ana Arka Planı kartında `Varsayılan`, `Resim` ya da `Video` seçin.
3. Resimde JPG/PNG/WebP; videoda MP4/WebM seçin. Video için isteğe bağlı fallback resmi yükleyin.
4. Karartmayı 0-100 arasında ayarlayın ve önizlemeyi kontrol edin.
5. **Kaydet / Yayınla** düğmesine basın. Loader bir sonraki config yüklemesinde yeni medyayı alır; build gerekmez.
6. **Varsayılana Dön** özel medyayı config'ten kaldırır ve artık kullanılmıyorsa storage dosyasını güvenli şekilde temizler.

Oyun cover/banner/icon medyası ile loader branding medyası ayrı storage ve işlem akışlarıdır.
