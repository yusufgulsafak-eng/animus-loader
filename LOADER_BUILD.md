# Windows Loader Build

## Development

Önce web API'nin çalıştığını ve loader/.env içindeki VITE_API_BASE_URL değerinin ona işaret ettiğini doğrulayın.

~~~powershell
Set-Location loader
npm install
npm run tauri dev
~~~

Vite arayüzü derler; Cargo ilk çalıştırmada Rust crate'lerini indirip native uygulamayı başlatır.

## Frontend kontrolü

~~~powershell
npm run build
npm test
~~~

## Rust testleri

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml
~~~

Testler path traversal, manifest sürümü ve Steam VDF ayrıştırma gibi negatif durumları içerir.

## EXE ve installer

~~~powershell
$env:VITE_API_BASE_URL = "https://animus-api.intelpol.workers.dev"
npm ci
npm test
npm run tauri build -- --config src-tauri/tauri.production.conf.json
~~~

Çıktılar loader/src-tauri/target/release ve bundle altındaki nsis/msi dizinlerinde oluşur. Product name ve application identifier loader/src-tauri/tauri.conf.json içindedir.

Production build yalnız HTTPS API originini kabul eder. Gerçek adresi build ortamında
`VITE_API_BASE_URL` olarak verin; geliştirme amaçlı `loader/.env` içindeki localhost
değeri release ayarı değildir.

Release giriş noktası Windows GUI subsystem kullanır. Bu nedenle başarılı release
binary açıldığında ayrı CMD/PowerShell konsolu göstermez; debug build davranışı
değişmez.

### Windows Application Control derlemeyi engellerse

`os error 4551` ile Cargo tarafından üretilen `build-script-build.exe` veya
proc-macro DLL'leri engelleniyorsa kaynak kodu değiştirmeyin ve Windows güvenlik
özelliklerini kapatmayın. Şu olay günlüğünü kontrol edin:

`Applications and Services Logs > Microsoft > Windows > CodeIntegrity > Operational`

Event 3033/3077 içindeki dosya ve Policy ID değerini sistem yöneticisine iletin.
Güvenli seçenek, kurumun yetkili Code Integrity politikasında build makinesi için
uygun geliştirme izni sağlamak veya build'i derlemeye izin verilen imzalı bir CI
ortamında almaktır.

## İmzalama

Production dağıtımında kod imzalama sertifikasını source code'a koymayın. Sertifikayı CI/build makinesinin güvenli certificate store'unda tutun. Tauri/Windows imzalama ortam değişkenlerini yalnız build sırasında sağlayın.

## Self-update

Patch güncellemesi ile loader güncellemesi ayrıdır. Loader /api/loader/latest endpoint'ini kontrol eder. Production self-update açılmadan önce:

1. Update imzalama key pair'i offline oluşturun.
2. Yalnız public key'i tauri.conf.json updater ayarına yazın.
3. Private key'i yalnız build secret olarak saklayın.
4. İmzalı installer ve signature dosyasını loader_versions kaydıyla yayınlayın.

Boş public key ile production self-update yayınlamayın.

## E-posta + Discord kapısı (yalnız Windows Loader)

- Kullanıcı girişinden sonra e-posta doğrulaması ve Discord bağlama işlemleri
  Loader arayüzündeki `HESAP GÜVENLİĞİ` ekranında yapılır. Web
  kütüphanesine ayrıca buton eklenmez.
- **E-posta doğrula:** `POST /api/auth/email/resend` ile yeni doğrulama
  bağlantısı istenir; bağlantı kullanıcının sistem tarayıcısında açılır.
- **Discord ile Bağlan:** `POST /api/auth/discord/link` resmi Discord
  OAuth2 sayfasını açar; zorunlu sunucu daveti: https://discord.gg/RMP6gGHpKK
  (Guild ID: 1480052266508746967).
- İki adım tamamlanana kadar Loader kütüphaneyi göstermez. Gerçek güvenlik
  sunucu tarafında indirme tokenı üretiminde ve dosya indirme isteğinde
  uygulanır. Her iki koruma da canlı olarak ayrıca test edilmelidir.
- Production frontend API hedefi `https://animus-api.intelpol.workers.dev/api` olmalıdır.
  `.env.production` ve Windows CI buna ayarlandı. Eski Workers originine
  derlenen exe yeni cPanel API sistemini kullanmaz.
- Bu zip **kaynak koddur**; Windows exe oluşturulduğu / canlı işleyiş
  test edildiği anlamına gelmez.
