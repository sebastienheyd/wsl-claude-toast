# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-05-07

### Added

- Tab title rewriting in `--hook` mode via OSC 0, formatted as
  `<emoji> <toast title> | <project>`, to identify the hosting terminal
  when several Claude Code sessions run side by side. Set
  `WCT_NO_TAB_TITLE=1` to opt out.
- Dedicated question toast for `elicitation_dialog` Notification events.
- `install-local.sh` script that builds and installs from the working
  tree via `cargo build --release`, for development without cutting a
  release.

### Changed

- Installation now detects an existing binary at the install target,
  unregisters the old hook, and re-runs `--install-hook` so the
  registered command always points at the freshly installed binary.
- Notification events without a `notification_type` are silently
  dropped, preventing a redundant toast emitted by Claude Code shortly
  after `Stop` when idle.
- Notification events that match no known type are silently dropped
  instead of producing a generic toast with placeholder strings.
- `auth_success` events and generic English fallback messages are now
  skipped so localized defaults take over.
- The Notification hook matcher is widened to `any`, so newly added
  event types are received.
- README install one-liners split into separate `wget`/`curl` blocks,
  giving each command its own copy button on GitHub.

### Removed

- Unused `toast.default.title` and `toast.default.default_msg` i18n
  keys from all locales.

## [0.1.0] - 2026-05-05

### Added

- Native Windows toasts via `Windows.UI.Notifications.ToastNotificationManager`,
  invoked through `powershell.exe` from WSL.
- Auto-detection of the `AppUserModelID` (priority on a personal
  `ClaudeCode.Notifier` AppID, then Claude → Windows PowerShell).
- Personal AppID registration without administrator rights, via a
  Start Menu `.lnk` shortcut plus `IPropertyStore`.
- Neutral sun icon (Twemoji ☀️) embedded as the default Action Center icon,
  with optional user-provided override at
  `%LOCALAPPDATA%\ClaudeCodeNotify\icon.{ico,png}`.
- Toast icons embedded into the binary (`include_bytes!`) and extracted at
  runtime to `%LOCALAPPDATA%\claude-notify\icons\`: `permission`, `stop`,
  `default`.
- `--hook` mode: reads the Claude Code JSON event from stdin and derives
  title, message, icon, and footer (git repo name when available, otherwise
  the full `cwd`).
- Idempotent install/uninstall of the hook in `~/.claude/settings.json`
  via `--install-hook` / `--uninstall-hook`.
- Localized output (toasts + CLI messages) in 5 languages: English, French,
  Spanish, German, Italian. Auto-detected from the Windows display language.
- `--icon` accepts a builtin name, a WSL/Windows path, or an `http(s)://` URL.
- `--footer` and `--app-id` flags for manual usage.

[Unreleased]: https://github.com/sebastienheyd/wsl-claude-toast/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/sebastienheyd/wsl-claude-toast/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/sebastienheyd/wsl-claude-toast/releases/tag/v0.1.0
