<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Şifremi Unuttum</title>
<link rel="stylesheet" href="/assets/app.css">
</head>
<body class="login-page">
<main class="login-card">
    <div class="brand large">
        <span class="brand-mark">A</span>
        <span>ANIMUS</span>
    </div>

    <h1>Şifremi Unuttum</h1>

    <?php if(!empty($message)): ?>
        <div class="alert"><?=$esc($message)?></div>
    <?php else: ?>
        <?php if(!empty($error)): ?>
            <div class="alert error"><?=$esc($error)?></div>
        <?php endif; ?>

        <p class="muted">
            Hesabınız varsa güvenli sıfırlama bağlantısı e-posta adresinize gönderilir.
        </p>

        <form method="post" action="/forgot-password">
            <input type="hidden" name="_csrf" value="<?=$esc($csrf)?>">
            <label>
                E-posta
                <input name="email" type="email" required autocomplete="email">
            </label>
            <button class="button primary wide">Bağlantı Gönder</button>
        </form>
    <?php endif; ?>

    <a class="button ghost wide" href="/">Ana Sayfa</a>
</main>
</body>
</html>
