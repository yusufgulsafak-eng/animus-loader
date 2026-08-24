<?php
declare(strict_types=1);

namespace App\Core;

final class View
{
    public static function render(string $view, array $viewData = []): never
    {
        extract($viewData, EXTR_SKIP);
        require WEB_ROOT . '/resources/views/' . $view . '.php';
        exit;
    }
}
