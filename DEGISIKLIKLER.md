# Animus Loader — Silme Altyapısı ve Backend Sağlamlaştırma

Bu sürümde sistemde hiç olmayan **kalıcı silme** akışı eklendi ve backend'in
kırık olan üç yapısal noktası onarıldı. Tüm değişiklikler gerçek MariaDB 10.11
üzerinde 59 uçtan uca test ile doğrulandı.

---

## 1. Kurulum (sırayla)

```bash
# 1) Yeni dosyaları sunucuya yükle (aşağıdaki dosya listesi)
# 2) Migration'ları çalıştır
php web/scripts/migrate.php

# 3) Karantina dizininin yazılabilir olduğundan emin ol
mkdir -p web/storage/trash && chmod 750 web/storage/trash

# 4) (Opsiyonel ama önerilir) cPanel'de gece cron'u tanımla
/usr/local/bin/php /home/KULLANICI/api/web/scripts/maintenance.php
```

`.env` dosyana iki yeni satır ekle:

```
TRASH_STORAGE_PATH=storage/trash
TRASH_RETENTION_DAYS=14
```

Migration'lar **idempotent**: kolonlar zaten varsa hiçbir şey yapmazlar,
iki kez çalıştırmak güvenlidir (test edildi).

---

## 2. Kritik hata: eksik migration

`AdminService::createPatchVersion()` ve `panelData()` şu iki kolona yazıp
okuyordu:

- `patch_archives.source_type`
- `patch_archives.external_url`

Ama bu kolonlar **hiçbir migration dosyasında tanımlı değildi.** Canlı
veritabanında elle eklenmiş olmalı; sıfırdan kurulan her sunucuda "yeni yama
oluştur" işlemi fatal hata ile düşerdi.

`007_patch_archive_sources.sql` bunu kalıcı olarak çözer.

**Aynı eksikliğin ikinci sonucu:** harici (PixelDrain) kaynaklı yamalar
indirilemiyordu. `ApiController::download()` diskte olmayan bir dosyayı
`readfile()` ile açmaya çalışıp `RuntimeException` fırlatıyordu. Artık harici
arşivlerde 302 yönlendirme yapılıyor ve indirme logu `completed` işaretleniyor.

---

## 3. Silme altyapısı — `DeletionService`

Eskiden silme işlemleri dağınıktı ve çoğu hiç yoktu:

| İşlem | Önce | Şimdi |
|---|---|---|
| `delete_patch_version` | **yoktu** | var |
| `delete_loader_version` | **yoktu** | var |
| `delete_subscription` | **yoktu** | var |
| `delete_user` | **yoktu** | var (super_admin'e özel) |
| `delete_game` | sadece API | her iki arayüzde |
| `delete_category` | sadece API | her iki arayüzde |
| `delete_announcement` / `delete_banner` | vardı | tek merkeze taşındı |

Tüm silme mantığı `app/Services/DeletionService.php` altında toplandı ve şu
kurallara göre çalışır:

### 3.1 Önce etki raporu, sonra onay

`describe_deletion` action'ı silmeden önce ne kaybedileceğini döner:

```json
{
  "entity": "game",
  "label": "Resident Evil 3 Remake",
  "blocking": ["Bu oyunun 2 adet YAYINDA sürümü var. Silmek loader istemcilerinde yamayı anında kaldırır."],
  "cascade": {
    "patch_versions": 5,
    "published_versions": 2,
    "archive_bytes": 1073741824,
    "download_logs_detached": 431
  },
  "requires_force": true
}
```

Panel bu raporu okuyup onay penceresinde gerçek rakamları gösterir. Rapor
destekleyen varlıklar: `game`, `patch_version`, `loader_version`, `category`,
`user`.

### 3.2 Yayındaki kayıt `force` olmadan silinmez

Aktif yayın, en güncel loader paketi veya oyuna bağlı kategori silinmek
istendiğinde işlem reddedilir. Kullanıcı ikinci bir onay verirse `force: true`
ile geçer.

### 3.3 Kanal devri otomatik

Bir kanalın aktif yayınını sildiğinde:

1. Aynı kanalda yayınlanmış bir önceki sürüm aranır.
2. Bulunursa `patch_release_channels` o sürüme devredilir ve sürüm
   `PUBLISHED` yapılır — **loader istemcileri boşluğa düşmez.**
3. Bulunamazsa kanal kaydı tamamen kaldırılır.

`patch_release_channels.active_patch_version_id` foreign key'i `ON DELETE
RESTRICT` olduğu için bu adım atlanamaz; eskiden `delete_patch_version` yazılmaya
kalkılsa doğrudan FK hatası verirdi.

### 3.4 Güvenlik kilitleri

- Kendi hesabını silemezsin.
- `super_admin` hesabını yalnız başka bir `super_admin` silebilir.
- Sistemde en az bir aktif admin kalmak zorunda.
- `delete_user` action'ı yalnız `super_admin` rolüne açık.

### 3.5 Geçmiş korunur

- `download_logs` **silinmez**, `patch_archive_id` NULL'a çekilerek anonimleşir.
  İndirme istatistiklerin bozulmaz.
- Silinen kaydın tam kopyası `audit_logs.before_json` içine yazılır.

---

## 4. Dosyalar kaybolmuyor — `StorageGc`

Silme artık `unlink()` çağırmıyor. Dosyalar `storage/trash/{alan}/{tarih}/`
altına **karantinaya** alınıyor ve `TRASH_RETENTION_DAYS` (varsayılan 14 gün)
sonunda kalıcı siliniyor. Yanlış silmede ZIP geri alınabilir.

Üç ek koruma:

1. **Dosya işlemi transaction dışında.** Önce DB commit edilir, sonra dosya
   taşınır. "DB silindi ama disk doldu" durumu oluşmaz.
2. **`storage_gc_queue` tablosu.** Dosya taşınamazsa kayıt kuyruğa düşer,
   bakım script'i (veya paneldeki buton) tekrar dener.
3. **Orphan tarama.** Veritabanında karşılığı olmayan dosyaları bulur.
   1 saatten yeni dosyalar (devam eden yükleme olabilir) atlanır.

---

## 5. Asıl mimari sorun: iki controller birbirinden kaymıştı

`AdminController` (web paneli) ve `ApiController` (senin ekran görüntüsündeki
loader istemcisi) **ayrı ayrı `match` blokları** tutuyordu. Listeler zamanla
uyuşmaz hale gelmişti:

- `update_user`, `save_subscription`, `create_loader_version`,
  `save_loader_config`, `save_branding_media` → sadece web panelinde
- `delete_game`, `delete_category`, `inspect_external_patch` → sadece API'de

Her ikisi de artık `app/Support/AdminActions.php` registry'sine yönleniyor.
Toplam **42 action**, tek liste. Bundan sonra eklenen her işlem otomatik olarak
iki arayüzde de çıkar — bu tür kaymalar bir daha oluşmaz.

---

## 6. Yol boyunca düzeltilen diğer hatalar

**`duplicateGame` bozuktu.** Veritabanından gelen `supported_stores` bir JSON
string'i; `saveGame` onu tekrar `json_encode` ediyor ve `"[\"manual\"]"` gibi
çift kodlanmış bir değer yazıyordu. Kopyalanan her oyunun mağaza listesi
bozuluyordu. Düzeltildi, ayrıca kategori bağlantıları ve audit kaydı eklendi.

**`setPatchStatus` asılı referans bırakıyordu.** Yayındaki bir sürümü
`DISABLED`/`ARCHIVED` yaptığında `patch_release_channels` hâlâ o sürümü
gösteriyordu. Katalog sorgusu `status='PUBLISHED'` filtrelediği için yama
sessizce kayboluyordu. Artık kanal ya bir önceki yayına devrediliyor ya da
kaldırılıyor — işlem transaction içinde ve `FOR UPDATE` kilidiyle.

---

## 7. Yeni Bakım / Depolama paneli

Sol menüye eklendi. İçeriği:

- Karantina durumu (bekleyen silme, başarısız silme, dosya sayısı, boyut)
- Bekleyen silmeleri tekrar dene
- Orphan dosya tara / karantinaya al
- Karantinayı kalıcı temizle
- Süresi dolmuş token temizliği
- Eski indirme loglarını sil (gün bazlı)
- Terk edilmiş draft sürümleri listele / sil (önce önizleme, sonra uygula)

Aynı işlemler `scripts/maintenance.php` ile cron'dan da çalışır:

```bash
php web/scripts/maintenance.php              # normal bakım
php web/scripts/maintenance.php --dry-run    # hiçbir şey silmeden rapor
php web/scripts/maintenance.php --purge-orphans
```

---

## 8. Değişen / eklenen dosyalar

### Yeni

```
web/database/007_patch_archive_sources.sql     Eksik kolonlar (kritik)
web/database/008_deletion_infrastructure.sql   GC kuyruğu + indeksler
web/app/Services/DeletionService.php           Tüm silme mantığı
web/app/Services/StorageGc.php                 Karantina, kuyruk, orphan tarama
web/app/Support/AdminActions.php               Tek action registry (42 action)
web/scripts/maintenance.php                    Cron bakım aracı
```

### Değişen

```
web/app/Controllers/AdminController.php   Registry'ye yönlendirildi
web/app/Controllers/ApiController.php     Registry + harici indirme düzeltmesi
web/app/Services/AdminService.php         3 hata düzeltmesi + silme devri
web/app/Services/PatchStorage.php         directory() erişimcisi
web/app/Services/LoaderStorage.php        directory() erişimcisi
web/app/Services/ImageStorage.php         directory() + absolutePath()
web/app/Services/BrandingMediaStorage.php directory() + absolutePathFromUrl()
web/resources/views/admin.php             Sil butonları + Bakım paneli
web/public/assets/admin.js                Silme akışı + bakım işlemleri
web/public/assets/admin-extended.css      Danger buton + panel stilleri
.env.example                              TRASH_* ayarları
```

---

## 9. Test sonuçları

MariaDB 10.11 + PHP 8.3, gerçek şema üzerinde:

```
== 1. Etki raporu ==                            4/4
== 2. Koruma kuralları ==                       5/5
== 3. Draft silme ==                            4/4
== 4. Aktif yayını force ile silme ==           9/9
== 5. Son yayını silme ==                       2/2
== 6. Oyun silme (cascade) ==                   5/5
== 7. Loader sürümü ==                          3/3
== 8. Kategori ==                               4/4
== 9. Kullanıcı ==                              3/3
== 10. Storage GC ==                            7/7
== 11. Token / log bakımı ==                    2/2
== 12. AdminActions registry ==                 7/7
== 13. Referans bütünlüğü ==                    4/4

PASS: 59   FAIL: 0
```

Ayrıca doğrulandı: migration'lar iki kez üst üste çalıştırıldığında hata
vermiyor; `maintenance.php` hem normal hem `--dry-run` modunda temiz çalışıyor.

---

## 10. Sırada ne var (öneriler)

Bu sürümde kapsam dışı bıraktığım, sonraki adım için not ettiklerim:

1. **`Yedek_ApiController.php`** ölü kod olarak duruyor. Route'lanmıyor ama
   sunucuda gereksiz; silinmesi temiz olur.
2. **`RateLimiter`** her istekte tüm tabloyu tarayıp `DELETE` çalıştırıyor.
   Yoğunlukta darboğaz olur; süresi dolmuş kayıtları bakım cron'una taşımak
   daha doğru.
3. **Geri alma (restore) ekranı.** Şu an audit log'daki `before_json` ile elle
   geri dönülebiliyor. Panelden tek tıkla restore için ayrı bir "çöp kutusu"
   ekranı yazılabilir.
4. **Toplu seçim.** Şu an silme tek tek. Yama listesinde checkbox ile toplu
   silme mantıklı bir sonraki adım.
