# ANIMUS — Public Worker API / Gizli origin yapılandırması

İstenen kullanıcıya açık URL: `https://animus-api.intelpol.workers.dev`

**DİKKAT:** Bu dosyaların güncellenmesi yalnızca Loader derleme URL'sini değiştirir. Mevcut Worker'ın cPanel'e **proxy** yaptığı doğrulanmadıkça üretime alma. Bu paket Cloudflare hesabına/cPanel'e otomatik bağlanmadı, canlı proxy testi yapılmadı.

## Yapılacaklar

1. Cloudflare Workers & Pages > mevcut `animus-api` Worker'ın mevcut kodunu yedekle. Mevcut Worker zaten proxy yapıyorsa onu kırma; `deployment/cloudflare/worker.mjs` yalnızca incelenebilecek örnek ters proxy'dir. Worker mevcut işlevlerinden farklıysa önceden birleştirilmeli.
2. Worker Settings > Variables and Secrets bölümünde `UPSTREAM_ORIGIN` adında bir **secret** tanımla. Değer gerçek cPanel HTTPS origin URL'si olmalı ve sonu `/` ile bitmeli. Değeri GitHub'a yazma. Örnek Worker, istemciden gelen istekleri 301/302 yönlendirme yapmadan origin'e iletir; absolute `Location` ve çerez `Domain` değerlerini public Worker hostuna uyarlar.
3. cPanel `public_html/api/.env` içindeki **APP_URL** değerini `https://animus-api.intelpol.workers.dev` yap **yalnızca** Worker yönlendirmesinin çalıştığı doğrulandıktan sonra. Bu, e-posta doğrulama, parola sıfırlama, indirme tokenı ve manifestte oluşturulan linkleri public hosttan üretir. Mail gönderen adresi kendi özel alan adını ifşa edecekse markaya ait uygun bir posta alan adı kullan.
4. Discord Developer Portal > OAuth2 > Redirects kısmına `https://animus-api.intelpol.workers.dev/auth/discord/callback` adresini ekle. Gerçek cPanel origin callback adresini kullanıcıya gösteren OAuth2 akışında kullanma. Redirect, cPanel APP_URL ve Worker URL birebir aynı olmalı.
5. CORS_ORIGINS değerinde `tauri://localhost,http://tauri.localhost,https://animus-api.intelpol.workers.dev` ve gerekli diğer güvenilir frontend originlerini bırak.
6. GitHub'daki Loader ve `.github/workflows` dosyalarını bu paketten güncelle. GitHub Actions release'i yeniden çalıştır. Eski EXE kendiliğinden değişmez.
7. Son test: Worker `GET /health` JSON 200; Loader login/register; gerçek e-posta linki Worker; gerçek Discord callback Worker; `download-token` URL'si Worker; gerçek izinli ve izinsiz yama indirmesi. **Hepsi geçmeden final kabul etme.**

## Sınırlar ve güvenlik

- Yalnızca Loader'da public Worker adresinin görünmesini sağlamak, gerçek sunucu adresini internetten tamamen gizlemez. DNS, eski bağlantılar, doğrudan HTTP endpointleri veya diğer web sayfaları origin'i açığa çıkarabilir. Katı gizlilik için origin'in doğrudan erişimini güvenlik duvarı veya Cloudflare Tunnel ile sınırlayıp özel admin erişimi planlamak gerekir.
- Origin'e doğrudan isteklerle indirmenin atlatılamadığını ayrıca doğrula; şu an yalnızca UI adresini değiştiriyoruz.
- Worker reverse proxy'de büyük ZIP dosyalarının **streaming** ve Range isteklerini test et; cPanel/Cloudflare transfer sınırları sorun olabilir.
- `.env`, DB parolası, Discord bot tokenı, APP_KEY, private SSH key veya cPanel origin hostname GitHub'a yüklenmemeli.
- Cloudflare Worker'ın mevcut kodunu otomatik değiştirmedik. Örnek proxy gerçek Worker üzerine körlemesine yapıştırılmamalı.
