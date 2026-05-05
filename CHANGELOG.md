# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/sebastienheyd/wsl-claude-toast/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/sebastienheyd/wsl-claude-toast/releases/tag/v0.1.0
