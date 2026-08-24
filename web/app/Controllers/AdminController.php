<?php
declare(strict_types=1);

namespace App\Controllers;

use App\Core\Csrf;
use App\Core\Env;
use App\Core\Http;
use App\Core\Session;
use App\Core\View;
use App\Services\AdminService;
use App\Services\AuthService;
use App\Support\AdminActions;
use DomainException;
use Throwable;

final class AdminController
{
    public function handle(string $method, string $path): never
    {
        $auth = new AuthService();

        if ($path === '/admin/login' && $method === 'GET') {
            View::render('admin-login', ['csrf' => Csrf::token()]);
        }
        if ($path === '/admin/login' && $method === 'POST') {
            $this->csrf($_POST['_csrf'] ?? null);
            try {
                $user = $auth->login((string)($_POST['email'] ?? ''), (string)($_POST['password'] ?? ''));
                if (!in_array($user['role'], ['admin', 'super_admin'], true)) {
                    throw new DomainException('Admin yetkisi gerekli.');
                }
                header('Location: /admin');
                exit;
            } catch (Throwable $error) {
                View::render('admin-login', ['csrf' => Csrf::token(), 'error' => $error->getMessage()]);
            }
        }

        if (!Session::isAdmin()) {
            header('Location: /admin/login');
            exit;
        }

        if ($path === '/admin/logout' && $method === 'POST') {
            $this->csrf($_POST['_csrf'] ?? null);
            $auth->logout();
            header('Location: /admin/login');
            exit;
        }
        if ($path === '/admin/action' && $method === 'POST') {
            $this->action();
        }

        View::render('admin', [
            'csrf' => Csrf::token(),
            'user' => Session::user(),
            'data' => (new AdminService())->panelData(),
        ]);
    }

    /**
     * Tüm admin işlemleri AdminActions registry'sine gider.
     * Web paneli ile loader istemcisi böylece aynı action listesini paylaşır.
     */
    private function action(): never
    {
        $this->csrf($_SERVER['HTTP_X_CSRF_TOKEN'] ?? ($_POST['_csrf'] ?? null));

        $body = Http::body();
        $action = (string)($body['action'] ?? '');
        $user = Session::user() ?? [];

        try {
            $result = AdminActions::dispatch($action, $body, $_FILES, $user);
            Http::json(['ok' => true, 'data' => $result]);
        } catch (DomainException $error) {
            Http::error($error->getMessage(), 422);
        } catch (Throwable $error) {
            error_log('Admin action error [' . $action . ']: ' . $error->getMessage());
            Http::error(
                Env::bool('APP_DEBUG') ? 'İşlem tamamlanamadı: ' . $error->getMessage() : 'Sunucu tarafında bir hata oluştu.',
                500
            );
        }
    }

    private function csrf(?string $token): void
    {
        if (!Csrf::verify($token)) {
            Http::error('CSRF doğrulaması başarısız.', 419);
        }
    }
}
