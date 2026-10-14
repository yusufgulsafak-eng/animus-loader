# ANIMUS Loader — cPanel uyumlu kaynak paketi

**İstenen arayüz:** E-posta doğrula + Discord ile Bağlan düğmeleri yalnızca Windows Loader'da.
Web kütüphanesinde bu özellikler için yeni düğme yapılmadı.

**Loader davranışı:** E-posta veya Discord hesabı bağlı değilse Loader'da "Hesap Güvenliği"
ekranı gösterilir. Davet: https://discord.gg/RMP6gGHpKK. Kullanıcı iki
kontrolü bitirip "Durumu Kontrol Et" dediğinde kütüphaneye geçebilir.

**API:** Production Loader yalnızca https://animus-api.intelpol.workers.dev/api adresini kullanır. `.env.production`, `.env.production.example`, Tauri production CSP
ve GitHub Windows workflow'ları aynı adresi kullanır.

**Sunucu tarafı:** Üyelik doğrulaması/indirme engeli Windows UI kontrolüyle
sınırlı değildir; cPanel API'nin doğrulama ve indirme kontrolleri de çalışmalıdır.
Bu pakette canlı sunucuda değişiklik yapılmadı.

**Geçen yerel kontroller:** 30 PHP testi; web/app ve web/public PHP sözdizimi;
Loader TypeScript sözdizimi ayrıştırması; kaynakta doğrulama kontrolü/endpointleri
ve production API URL tutarlılığı; bilinen paylaşılan gizli bilgilerin paket dışı kalması.

**Yapılamayan kontroller:** npm bağımlılıklarının tamamı bu ortamda erişilebilir
olmadığından `npm run build` ve `npm test` çalıştırılamadı. Windows Tauri EXE
ve installer derlenmedi. E-posta teslimi, canlı OAuth2 callback, Discord üye
sorgusu ve gerçek yama indirmesi burada uçtan uca test edilmedi.

**Windows derleme:** `LOADER_BUILD.md` talimatlarına bakın. GitHub Actions
workflow'ları da cPanel API URL'siyle ayarlanmıştır. Kaynak ZIP'i doğrudan
cPanel'e yüklemeyin; Loader için yeni Windows kurulum dosyası oluşturun.

**Önemli:** Eski kurulum dosyaları kendiliğinden güncellenmez. Kaynakları GitHub üzerinde güncelleyip yeni bir release derleyin. Worker, cPanel API'sine proxy olarak bağlanmadan Loader'ın çalıştığı doğrulanmış sayılmaz. Gerçek testler tamamlanmadan güvenlik
uygulamasını canlıda tam doğrulanmış saymayın.


**Yeni Worker dönüşü:** https://animus-api.intelpol.workers.dev görünür endpoint. cPanel APP_URL ve Discord Redirect URI güncellenmeden kullanıcı linkleri gizlenmez. Ayrıntılar WORKER_PUBLIC_API_KURULUM.md.
