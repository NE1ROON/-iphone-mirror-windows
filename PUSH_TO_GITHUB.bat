@echo off
chcp 65001 >nul
echo ========================================
echo  Push to GitHub - iPhone Mirror Windows
echo ========================================
echo.
echo ШАГ 1: Создайте репозиторий на GitHub
echo   1. Откройте https://github.com/new
echo   2. Имя: iphone-mirror-windows
echo   3. Public или Private - на ваш выбор
echo   4. НЕ добавляйте README, .gitignore, license (уже есть)
echo   5. Нажмите "Create repository"
echo.
echo ШАГ 2: Вставьте ссылку на репозиторий ниже
echo   Пример: https://github.com/ВАШ_ЮЗЕРНЕЙМ/iphone-mirror-windows.git
echo.
set /p REPO_URL="URL репозитория: "
if "%REPO_URL%"=="" (
    echo URL не указан. Выход.
    pause
    exit /b 1
)

echo.
echo [GIT] Adding remote origin...
git remote add origin %REPO_URL%

echo [GIT] Pushing to GitHub...
git branch -M main
git push -u origin main

if %errorlevel% equ 0 (
    echo.
    echo ========================================
    echo  SUCCESS! Репозиторий загружен.
    echo ========================================
    echo.
    echo Теперь:
    echo   1. Зайдите на GitHub -> ваш репозиторий -> Actions
    echo   2. Дождитесь завершения билда (зелёная галочка)
    echo   3. Нажмите на последний workflow -> Artifacts -> iphone-mirror-windows-msi
    echo   4. Скачайте ZIP, распакуйте -> получите .msi установщик
    echo.
) else (
    echo.
    echo [ERROR] Push failed. Проверьте URL и доступы.
)

pause