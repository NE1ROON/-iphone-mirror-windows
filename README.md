# iPhone Mirror Windows

Обёртка для **MirrorDrop** (и других AirPlay-ресиверов), которая делает окно зеркалирования iPhone на Windows:
- **Без рамки** (frameless)
- **Всегда поверх окон** (always-on-top)
- **Перетаскиваемое** за любую область
- С **скруглёнными углами** и тенью (как на macOS)

## 📥 Скачать готовый .msi

> **Не хотите собирать сами?** Перейдите в **Actions** → последний успешный билд → **Artifacts** → скачайте `iphone-mirror-windows-msi.zip` → распакуйте и запустите `.msi`.

## 🚀 Быстрый старт

1. **Установите MirrorDrop** (бесплатно, open source, **рекомендуется для Windows**):
   - Скачайте: https://mirrordrop.app/windows → **Download for Windows**
   - Запустите `MirrorDrop.exe` (установка не требуется)

   **Альтернативы** (если MirrorDrop не заработал):
   - **AirPlay-Windows**: https://github.com/Thomas-lab17/AirPlay-Windows/releases
   - **AirPlay-Receiver**: https://github.com/ZC635/AirPlay-Receiver/releases
   - **Reflector 4** (платный, триал 7 дней): https://www.airsquirrels.com/reflector/

2. **Запустите MirrorDrop** и подключите iPhone:
   - На iPhone: «Экран» → «MirrorDrop» (или «AirPlay» → «MirrorDrop»)
   - Экран iPhone появится в окне MirrorDrop

3. **Установите этот .msi** (скачанный из Actions)
   - В трее появится иконка 📱

4. **Настройте** (правый клик на иконке → «Настройки…»):
   - **Ресивер**: **MirrorDrop** (по умолчанию)
   - **Размер окна**: 400×720 (под iPhone 14/15/16) или ваш
   - ✅ **Автозапуск с Windows**
   - ✅ **Показывать при запуске**

5. **Готово!** Окно MirrorDrop теперь:
   - Без заголовка и рамок
   - Всегда поверх других окон
   - Перетаскивается за любое место (как виджет)
   - С красивой тенью и скруглёнными углами

## ⌨️ Горячие клавиши (планируется)

| Сочетание | Действие |
|-----------|----------|
| `Win` + `Shift` + `M` | Показать/скрыть окно зеркалирования |
| `Win` + `Shift` + `S` | Открыть настройки |

## 🛠 Сборка самим (если нужно)

### Требования
- Windows 10/11
- Rust (stable): `winget install Rustlang.Rust.GNU`
- Visual Studio Build Tools (C++ workload) или `winget install Microsoft.VisualStudio.2022.BuildTools`

### Команды
```bash
cd iphone-mirror-windows
cargo tauri build
```
Результат: `src-tauri/target/release/bundle/msi/iphone-mirror-windows_1.0.0_x64.msi`

## 🔧 Поддерживаемые ресиверы

| Ресивер | Цена | Статус | Скачать |
|---------|------|--------|---------|
| **MirrorDrop** | Бесплатно | ✅ **Основной (Windows)** | [mirrordrop.app/windows](https://mirrordrop.app/windows) |
| **AirPlay-Windows** | Бесплатно (GPLv3) | ✅ Альтернатива | [GitHub Releases](https://github.com/Thomas-lab17/AirPlay-Windows/releases) |
| **AirPlay-Receiver** | Бесплатно (GPLv3) | ✅ Альтернатива | [GitHub Releases](https://github.com/ZC635/AirPlay-Receiver/releases) |
| **Reflector 4** | Платный ($19.99) | ✅ Работает | [airsquirrels.com](https://www.airsquirrels.com/reflector/) |
| **LonelyScreen** | Бесплатно | ⚠️ Устарел | [lonely-screen.com](https://www.lonely-screen.com/) |

> **Важно:** Bezel работает **только на macOS**. На Windows используйте MirrorDrop или альтернативы выше.

## 🐛 Решение проблем

**Окно не находится?**
1. Убедитесь, что MirrorDrop **запущен** и iPhone **активно зеркалируется**
2. В настройках нажмите **«Проверить доступность»**
3. Перезапустите MirrorDrop, затем это приложение
4. Попробуйте другой ресивер в настройках (AirPlay-Windows / AirPlay-Receiver)

**Окно мерцает / не перетаскивается?**
- Перезапустите приложение (правый клик → Выход → запустите снова)
- Проверьте, не конфликтует ли с другими утилитами окон (PowerToys FancyZones и т.д.)

**Антивирус блокирует?**
- Добавьте папку установки в исключения
- Это open source, код полностью прозрачен

## 📄 Лицензия

MIT — делайте что хотите.

## 🤝 Присоединяйтесь

- Issues / PRs welcome
- Идеи: клик-сквозь (click-through), зум колёсиком, запись экрана, несколько iPhone одновременно

---

**Сделано с ❤️ на Tauri 2 + Rust**