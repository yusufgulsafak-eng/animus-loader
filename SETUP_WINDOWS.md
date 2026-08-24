# Windows Geliştirme Ortamı

Kaynak kod üretimi dependency olmasa da tamamlanmıştır. Yerel geliştirme ve EXE oluşturmak için aşağıdakileri kurun.

## Gerekenler

1. PHP 8.2 veya üzeri:
   - pdo_mysql
   - mbstring
   - fileinfo
   - zip
   - openssl
2. MySQL 8 veya güncel MariaDB.
3. Node.js LTS ve npm.
4. Rust stable toolchain ve Cargo.
5. Microsoft Visual Studio Build Tools 2022:
   - Desktop development with C++
   - Windows 10/11 SDK
6. Microsoft Edge WebView2 Runtime.

Git zorunlu değildir.

## Kontrol komutları

~~~powershell
php --version
php -m
node --version
npm --version
rustc --version
cargo --version
~~~

## Ortam dosyaları

Repository kökünde:

~~~powershell
Copy-Item .env.example .env
Copy-Item loader/.env.example loader/.env
~~~

.env içine MySQL ve gerçek APP_URL bilgilerini girin. APP_KEY için 32 rastgele bayt üretip base64 biçiminde saklayın:

~~~powershell
php -r "echo 'base64:'.base64_encode(random_bytes(32)), PHP_EOL;"
~~~

Production secret dosyasını paylaşmayın.

