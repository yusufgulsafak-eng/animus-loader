# Windows Loader Build

## Development

Önce web API'nin çalıştığını ve loader/.env içindeki VITE_API_URL değerinin ona işaret ettiğini doğrulayın.

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
$env:VITE_API_BASE_URL = https://api.example.com
npm run tauri build
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
