# Design: Homebrew Cask (замість Formula)

**Дата**: 2026-02-22
**Статус**: Затверджено

## Проблема

`brew install itsserbin/tap/ferrum` встановлює лише бінарник у `/usr/local/bin/ferrum`. Додаток недоступний через Spotlight, Launchpad, Dock — бо `.app` бандл не потрапляє в `/Applications`.

## Рішення

Замінити Homebrew Formula на Homebrew Cask. Cask встановлює `.dmg` → `.app` в `/Applications` + CLI-симлінк `ferrum`.

## Компоненти

### 1. Новий Cask-шаблон

**Файл**: `installer/homebrew/ferrum.cask.rb.template`

```ruby
cask "ferrum" do
  arch arm: "aarch64", intel: "x86_64"

  version "${VERSION}"

  url "https://github.com/itsserbin/ferrum/releases/download/v#{version}/ferrum-#{arch}-apple-darwin.dmg"

  sha256 arm:   "${SHA256_ARM64}",
         intel: "${SHA256_X86_64}"

  name "Ferrum"
  desc "GPU-accelerated terminal emulator"
  homepage "https://github.com/itsserbin/ferrum"

  app "Ferrum.app"
  binary "#{appdir}/Ferrum.app/Contents/MacOS/Ferrum"

  zap trash: [
    "~/Library/Application Support/ferrum",
    "~/Library/Preferences/com.ferrum.terminal.plist",
  ]
end
```

### 2. Видалити Formula-шаблон

`installer/homebrew/ferrum.rb.template` — видалити.

### 3. Зміни в CI (`build-installers.yml`)

У release job:

- Додати крок підрахунку SHA256 для `ferrum-aarch64-apple-darwin.dmg` і `ferrum-x86_64-apple-darwin.dmg`
- Експортувати `SHA256_ARM64`, `SHA256_X86_64` у env
- Генерувати Cask: `envsubst < installer/homebrew/ferrum.cask.rb.template > Casks/ferrum.rb`
- Публікувати до tap-репо у `Casks/ferrum.rb` (замість `Formula/ferrum.rb`)

### 4. README.md

Команда установки:
```
# Було:
brew install itsserbin/tap/ferrum

# Стало:
brew install --cask itsserbin/tap/ferrum
```

Команду `xattr -cr /Applications/Ferrum.app` після установки залишити (ad-hoc підпис).

## Результат для користувача

```bash
brew install --cask itsserbin/tap/ferrum
# → /Applications/Ferrum.app  (доступно з Spotlight, Launchpad, Dock)
# → /usr/local/bin/ferrum      (CLI-симлінк)
```

## Обмеження

- Зміна breaking: існуючі Formula-установки не перейдуть автоматично. Потрібно `brew uninstall ferrum && brew install --cask itsserbin/tap/ferrum`.
- Cask підтримує тільки macOS. Для Linux Homebrew-установка (`brew install` без `--cask`) більше не діє — Linux-користувачі використовують `.deb`/`.rpm`.
