use std::collections::HashMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use clap::Parser;
use serde_json::{json, Value};

const I18N_EN: &str = include_str!("../assets/i18n/en.json");
const I18N_FR: &str = include_str!("../assets/i18n/fr.json");
const I18N_ES: &str = include_str!("../assets/i18n/es.json");
const I18N_DE: &str = include_str!("../assets/i18n/de.json");
const I18N_IT: &str = include_str!("../assets/i18n/it.json");

#[derive(Copy, Clone)]
enum Lang {
    En,
    Fr,
    Es,
    De,
    It,
}

impl Lang {
    fn detect() -> Self {
        let out = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; (Get-Culture).TwoLetterISOLanguageName",
            ])
            .output();
        let code = out
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_lowercase())
            .unwrap_or_default();
        match code.as_str() {
            "fr" => Lang::Fr,
            "es" => Lang::Es,
            "de" => Lang::De,
            "it" => Lang::It,
            _ => Lang::En,
        }
    }

    fn raw(self) -> &'static str {
        match self {
            Lang::En => I18N_EN,
            Lang::Fr => I18N_FR,
            Lang::Es => I18N_ES,
            Lang::De => I18N_DE,
            Lang::It => I18N_IT,
        }
    }
}

struct Strings {
    map: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl Strings {
    fn load(lang: Lang) -> Self {
        let map = serde_json::from_str(lang.raw()).unwrap_or_default();
        let fallback = serde_json::from_str(I18N_EN).unwrap_or_default();
        Self { map, fallback }
    }

    fn get(&self, key: &str) -> String {
        self.map
            .get(key)
            .or_else(|| self.fallback.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    fn fmt(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut s = self.get(key);
        for (name, val) in args {
            s = s.replace(&format!("{{{name}}}"), val);
        }
        s
    }
}

static STRINGS: OnceLock<Strings> = OnceLock::new();

fn strings() -> &'static Strings {
    STRINGS.get_or_init(|| Strings::load(Lang::detect()))
}

fn t(key: &str) -> String {
    strings().get(key)
}

fn tf(key: &str, args: &[(&str, &str)]) -> String {
    strings().fmt(key, args)
}

const CANDIDATE_APP_NAMES: &[&str] = &[
    "Claude",
    "Windows PowerShell",
    "PowerShell",
    "Windows Terminal",
];

const BUILTIN_ICONS: &[(&str, &[u8])] = &[
    ("permission", include_bytes!("../assets/permission.png")),
    ("stop", include_bytes!("../assets/stop.png")),
    ("default", include_bytes!("../assets/default.png")),
];

const SUN_ICO: &[u8] = include_bytes!("../assets/sun.ico");
const PERSONAL_APP_ID: &str = "ClaudeCode.Notifier";
const PERSONAL_APP_NAME: &str = "Claude Code";
const CUSTOM_ICON_DIR: &str = "ClaudeCodeNotify";

const PS_TEMPLATE: &str = r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null

$title = @'
__TITLE__
'@
$message = @'
__MESSAGE__
'@
$iconSrc = @'
__ICON__
'@
$footer = @'
__FOOTER__
'@

$iconXml = ''
if ($iconSrc) {
    $iconXml = "<image placement='appLogoOverride' src='$([System.Security.SecurityElement]::Escape($iconSrc))'/>"
}
$footerXml = ''
if ($footer) {
    $footerXml = "<text placement='attribution'>$([System.Security.SecurityElement]::Escape($footer))</text>"
}

$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml(@"
<toast>
    <visual>
        <binding template="ToastGeneric">
            $iconXml
            <text>$([System.Security.SecurityElement]::Escape($title))</text>
            <text>$([System.Security.SecurityElement]::Escape($message))</text>
            $footerXml
        </binding>
    </visual>
</toast>
"@)

$toast = New-Object Windows.UI.Notifications.ToastNotification $xml
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('__APP_ID__').Show($toast)
"#;

const PS_APPID_PINVOKE: &str = r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

namespace AppIdLib {
    [StructLayout(LayoutKind.Sequential, Pack = 4)]
    public struct PROPERTYKEY {
        public Guid fmtid;
        public uint pid;
    }

    [StructLayout(LayoutKind.Explicit)]
    public struct PROPVARIANT {
        [FieldOffset(0)] public ushort vt;
        [FieldOffset(2)] public ushort wReserved1;
        [FieldOffset(4)] public ushort wReserved2;
        [FieldOffset(6)] public ushort wReserved3;
        [FieldOffset(8)] public IntPtr pwszVal;
        [FieldOffset(8)] public long longVal;
    }

    [ComImport]
    [Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IPropertyStore {
        void GetCount(out uint cProps);
        void GetAt(uint iProp, out PROPERTYKEY pkey);
        void GetValue(ref PROPERTYKEY key, out PROPVARIANT pv);
        void SetValue(ref PROPERTYKEY key, ref PROPVARIANT pv);
        void Commit();
    }

    public static class Shell {
        [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
        public static extern void SHGetPropertyStoreFromParsingName(
            [MarshalAs(UnmanagedType.LPWStr)] string pszPath,
            IntPtr pbc,
            int flags,
            ref Guid riid,
            [MarshalAs(UnmanagedType.Interface)] out IPropertyStore ppv);

        private static readonly Guid IID_IPropertyStore = new Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99");
        private static readonly PROPERTYKEY PKEY_AppUserModel_ID = new PROPERTYKEY {
            fmtid = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"),
            pid = 5
        };

        public static void SetAppId(string lnkPath, string appId) {
            IPropertyStore store;
            Guid iid = IID_IPropertyStore;
            SHGetPropertyStoreFromParsingName(lnkPath, IntPtr.Zero, 2, ref iid, out store);
            PROPVARIANT pv = new PROPVARIANT();
            pv.vt = 31;
            pv.pwszVal = Marshal.StringToCoTaskMemUni(appId);
            PROPERTYKEY key = PKEY_AppUserModel_ID;
            store.SetValue(ref key, ref pv);
            store.Commit();
            Marshal.FreeCoTaskMem(pv.pwszVal);
            Marshal.ReleaseComObject(store);
        }

        public static string GetAppId(string lnkPath) {
            IPropertyStore store;
            Guid iid = IID_IPropertyStore;
            try {
                SHGetPropertyStoreFromParsingName(lnkPath, IntPtr.Zero, 0, ref iid, out store);
            } catch { return null; }
            PROPVARIANT pv;
            PROPERTYKEY key = PKEY_AppUserModel_ID;
            store.GetValue(ref key, out pv);
            string result = (pv.vt == 31 && pv.pwszVal != IntPtr.Zero) ? Marshal.PtrToStringUni(pv.pwszVal) : null;
            Marshal.ReleaseComObject(store);
            return result;
        }
    }
}
"@ -ErrorAction SilentlyContinue | Out-Null
"#;

const PS_REGISTER: &str = r#"
$appId   = $env:NOTIFY_APP_ID
$appName = $env:NOTIFY_APP_NAME
$target  = $env:NOTIFY_TARGET
$icon    = $env:NOTIFY_ICON
$lnk = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$appName.lnk"

$existing = $null
$existingIcon = $null
if (Test-Path -LiteralPath $lnk) {
    try { $existing = [AppIdLib.Shell]::GetAppId($lnk) } catch {}
    try {
        $ws0 = New-Object -ComObject WScript.Shell
        $sc0 = $ws0.CreateShortcut($lnk)
        $existingIcon = $sc0.IconLocation
    } catch {}
}

$ws = New-Object -ComObject WScript.Shell
$sc = $ws.CreateShortcut($lnk)
$sc.TargetPath = $target
if ($icon) { $sc.IconLocation = "$icon,0" }
$sc.Save()

[AppIdLib.Shell]::SetAppId($lnk, $appId)

$wantIcon = if ($icon) { "$icon,0" } else { "" }
if ($existing -eq $appId -and $existingIcon -eq $wantIcon) {
    Write-Output "ALREADY"
    exit 0
}

Start-Sleep -Seconds 5
Write-Output "INSTALLED"
"#;

const PS_UNREGISTER: &str = r#"
$appId   = $env:NOTIFY_APP_ID
$appName = $env:NOTIFY_APP_NAME
$lnk = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$appName.lnk"

if (-not (Test-Path -LiteralPath $lnk)) {
    Write-Output "ABSENT"
    exit 0
}

$existing = $null
try { $existing = [AppIdLib.Shell]::GetAppId($lnk) } catch {}
if ($existing -ne $appId) {
    Write-Output "FOREIGN"
    exit 0
}

Remove-Item -LiteralPath $lnk -Force
Write-Output "REMOVED"
"#;

const PS_DETECT: &str = r#"
$appId   = $env:NOTIFY_APP_ID
$appName = $env:NOTIFY_APP_NAME
$lnk = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$appName.lnk"

if (-not (Test-Path -LiteralPath $lnk)) { exit 1 }
try { $existing = [AppIdLib.Shell]::GetAppId($lnk) } catch { exit 1 }
if ($existing -eq $appId) { Write-Output $appId; exit 0 }
exit 1
"#;

#[derive(Parser, Debug)]
#[command(version, about = "Notification Windows depuis WSL.")]
struct Args {
    /// Titre de la notification (omis en mode --hook)
    title: Option<String>,

    /// Corps de la notification (omis en mode --hook)
    message: Option<String>,

    /// Lit un événement JSON Claude Code sur stdin
    #[arg(long)]
    hook: bool,

    /// Installe le hook dans ~/.claude/settings.json puis quitte
    #[arg(long)]
    install_hook: bool,

    /// Retire le hook de ~/.claude/settings.json puis quitte
    #[arg(long)]
    uninstall_hook: bool,

    /// AppUserModelID à utiliser (auto-détecté si omis)
    #[arg(long)]
    app_id: Option<String>,

    /// Chemin (WSL ou Windows), URL, ou nom intégré (permission, stop, default)
    #[arg(long)]
    icon: Option<String>,

    /// Texte de pied de notification (attribution)
    #[arg(long)]
    footer: Option<String>,
}

struct HookPayload {
    title: String,
    message: String,
    icon: String,
    footer: String,
}

fn git_project_name(cwd: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", cwd, "rev-parse", "--show-toplevel"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let toplevel = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if toplevel.is_empty() {
        return None;
    }
    Path::new(&toplevel)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
}

fn parse_hook_input() -> Result<HookPayload, String> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| tf("err.stdin.read", &[("error", &e.to_string())]))?;

    let json: Value = serde_json::from_str(&buf)
        .map_err(|e| tf("err.stdin.invalid_json", &[("error", &e.to_string())]))?;

    let event = json
        .get("hook_event_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let notif_type = json
        .get("notification_type")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut message = json
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let cwd = json.get("cwd").and_then(Value::as_str).unwrap_or("");

    let (title, icon, default_msg) = if notif_type == "permission_prompt" {
        (
            t("toast.permission.title"),
            "permission",
            t("toast.permission.default_msg"),
        )
    } else if event == "Stop" {
        (t("toast.stop.title"), "stop", t("toast.stop.default_msg"))
    } else {
        (
            t("toast.default.title"),
            "default",
            t("toast.default.default_msg"),
        )
    };

    if message.is_empty() {
        message = default_msg.to_string();
    }
    message = message.replace('\r', "");
    if message.chars().count() > 200 {
        message = message.chars().take(200).collect();
    }

    let footer = if cwd.is_empty() {
        String::new()
    } else {
        match git_project_name(cwd) {
            Some(name) => name,
            None => cwd.to_string(),
        }
    };

    Ok(HookPayload {
        title: title.to_string(),
        message,
        icon: icon.to_string(),
        footer,
    })
}

fn run_powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", script])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| tf("err.powershell.spawn", &[("error", &e.to_string())]))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(tf("err.powershell.failed", &[("stderr", &stderr)]));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn detect_app_id() -> Result<String, String> {
    if let Some(personal) = detect_personal_appid() {
        return Ok(personal);
    }

    let candidates = CANDIDATE_APP_NAMES
        .iter()
        .map(|n| format!("'{n}'"))
        .collect::<Vec<_>>()
        .join(",");

    let script = format!(
        "$apps = Get-StartApps; \
         foreach ($name in @({candidates})) {{ \
           $match = $apps | Where-Object {{ $_.Name -eq $name }} | Select-Object -First 1; \
           if ($match) {{ Write-Output $match.AppID; break }} \
         }}"
    );

    let app_id = run_powershell(&script)?;
    if app_id.is_empty() {
        return Err(t("err.no_appid").to_string());
    }
    Ok(app_id)
}

fn wslpath_to_windows(linux_path: &str) -> Result<String, String> {
    let output = Command::new("wslpath")
        .args(["-w", linux_path])
        .output()
        .map_err(|e| tf("err.wslpath.spawn", &[("error", &e.to_string())]))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(tf("err.wslpath.failed", &[("stderr", &stderr)]));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn wslpath_to_linux(win_path: &str) -> Result<String, String> {
    let output = Command::new("wslpath")
        .args(["-u", win_path])
        .output()
        .map_err(|e| tf("err.wslpath.spawn", &[("error", &e.to_string())]))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(tf("err.wslpath.failed", &[("stderr", &stderr)]));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read_windows_env(name: &str) -> Result<String, String> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Write-Output $env:{name}"
            ),
        ])
        .output()
        .map_err(|e| tf("err.powershell.spawn", &[("error", &e.to_string())]))?;
    if !output.status.success() {
        return Err(tf("err.read_env", &[("name", name)]));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn windows_local_appdata() -> Result<String, String> {
    read_windows_env("LOCALAPPDATA")
}

fn png_wrap_as_ico(png_data: &[u8]) -> Vec<u8> {
    let png_len = png_data.len() as u32;
    let mut ico = Vec::with_capacity(22 + png_data.len());
    ico.extend_from_slice(&[0, 0]);
    ico.extend_from_slice(&[1, 0]);
    ico.extend_from_slice(&[1, 0]);
    ico.push(0);
    ico.push(0);
    ico.push(0);
    ico.push(0);
    ico.extend_from_slice(&[1, 0]);
    ico.extend_from_slice(&[32, 0]);
    ico.extend_from_slice(&png_len.to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(png_data);
    ico
}

fn resolve_appid_icon() -> Result<String, String> {
    let win_local = windows_local_appdata()?;
    let cache_win_dir = format!(r"{win_local}\claude-notify\icons");
    let cache_linux_dir = wslpath_to_linux(&cache_win_dir)?;
    std::fs::create_dir_all(&cache_linux_dir)
        .map_err(|e| tf("err.create_icon_cache", &[("error", &e.to_string())]))?;

    let custom_win_dir = format!(r"{win_local}\{CUSTOM_ICON_DIR}");
    let custom_linux_dir = wslpath_to_linux(&custom_win_dir)?;
    let custom_ico_linux = format!("{custom_linux_dir}/icon.ico");
    let custom_png_linux = format!("{custom_linux_dir}/icon.png");

    if Path::new(&custom_ico_linux).exists() {
        return Ok(format!(r"{custom_win_dir}\icon.ico"));
    }

    if Path::new(&custom_png_linux).exists() {
        let png = std::fs::read(&custom_png_linux)
            .map_err(|e| tf("err.read_custom_icon", &[("error", &e.to_string())]))?;
        let ico_bytes = png_wrap_as_ico(&png);
        let cache_linux_file = format!("{cache_linux_dir}/custom.ico");
        let needs_write = !Path::new(&cache_linux_file).exists()
            || std::fs::metadata(&cache_linux_file)
                .map(|m| m.len())
                .unwrap_or(0)
                != ico_bytes.len() as u64;
        if needs_write {
            std::fs::write(&cache_linux_file, &ico_bytes)
                .map_err(|e| tf("err.write_ico", &[("error", &e.to_string())]))?;
        }
        return Ok(format!(r"{cache_win_dir}\custom.ico"));
    }

    let cache_linux_file = format!("{cache_linux_dir}/sun.ico");
    if !Path::new(&cache_linux_file).exists()
        || std::fs::metadata(&cache_linux_file)
            .map(|m| m.len())
            .unwrap_or(0)
            != SUN_ICO.len() as u64
    {
        std::fs::write(&cache_linux_file, SUN_ICO)
            .map_err(|e| tf("err.write_ico", &[("error", &e.to_string())]))?;
    }

    Ok(format!(r"{cache_win_dir}\sun.ico"))
}

fn run_powershell_with_env(script: &str, envs: &[(&str, &str)]) -> Result<String, String> {
    let tmp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let tmp_path = tmp_dir.join(format!("notify-ps-{pid}.ps1"));
    std::fs::write(&tmp_path, script)
        .map_err(|e| tf("err.write_ps_script", &[("error", &e.to_string())]))?;
    let win_path = wslpath_to_windows(&tmp_path.to_string_lossy())?;

    let mut cmd = Command::new("powershell.exe");
    cmd.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        &win_path,
    ])
    .stdin(Stdio::null());

    let names: Vec<&str> = envs.iter().map(|(k, _)| *k).collect();
    let wslenv = std::env::var("WSLENV").unwrap_or_default();
    let combined = if wslenv.is_empty() {
        names.join(":")
    } else {
        format!("{wslenv}:{}", names.join(":"))
    };
    cmd.env("WSLENV", combined);

    for (k, v) in envs {
        cmd.env(k, v);
    }
    let result = cmd.output();
    let _ = std::fs::remove_file(&tmp_path);
    let output = result.map_err(|e| tf("err.powershell.spawn", &[("error", &e.to_string())]))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(tf("err.powershell.failed", &[("stderr", &stderr)]));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn register_personal_appid() -> Result<bool, String> {
    let exe =
        std::env::current_exe().map_err(|e| tf("err.locate_exe", &[("error", &e.to_string())]))?;
    let target_win = wslpath_to_windows(&exe.to_string_lossy())?;
    let icon_win = resolve_appid_icon()?;

    let script = format!("{}{}", PS_APPID_PINVOKE, PS_REGISTER);
    let result = run_powershell_with_env(
        &script,
        &[
            ("NOTIFY_APP_ID", PERSONAL_APP_ID),
            ("NOTIFY_APP_NAME", PERSONAL_APP_NAME),
            ("NOTIFY_TARGET", &target_win),
            ("NOTIFY_ICON", &icon_win),
        ],
    )?;

    let last = result.lines().last().unwrap_or("").trim();
    Ok(last == "INSTALLED")
}

fn unregister_personal_appid() -> Result<bool, String> {
    let script = format!("{}{}", PS_APPID_PINVOKE, PS_UNREGISTER);
    let result = run_powershell_with_env(
        &script,
        &[
            ("NOTIFY_APP_ID", PERSONAL_APP_ID),
            ("NOTIFY_APP_NAME", PERSONAL_APP_NAME),
        ],
    )?;
    let last = result.lines().last().unwrap_or("").trim();
    Ok(last == "REMOVED")
}

fn detect_personal_appid() -> Option<String> {
    let script = format!("{}{}", PS_APPID_PINVOKE, PS_DETECT);
    run_powershell_with_env(
        &script,
        &[
            ("NOTIFY_APP_ID", PERSONAL_APP_ID),
            ("NOTIFY_APP_NAME", PERSONAL_APP_NAME),
        ],
    )
    .ok()
    .filter(|s| !s.trim().is_empty())
    .map(|s| s.lines().last().unwrap_or("").trim().to_string())
    .filter(|s| s == PERSONAL_APP_ID)
}

fn extract_builtin_icon(name: &str) -> Result<String, String> {
    let bytes = BUILTIN_ICONS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| *b)
        .ok_or_else(|| tf("err.unknown_builtin_icon", &[("name", name)]))?;

    let win_local = windows_local_appdata()?;
    let win_dir = format!(r"{win_local}\claude-notify\icons");
    let linux_dir = wslpath_to_linux(&win_dir)?;
    std::fs::create_dir_all(&linux_dir)
        .map_err(|e| tf("err.create_icon_cache", &[("error", &e.to_string())]))?;

    let linux_file = format!("{linux_dir}/{name}.png");
    if !Path::new(&linux_file).exists() {
        std::fs::write(&linux_file, bytes)
            .map_err(|e| tf("err.write_icon", &[("error", &e.to_string())]))?;
    }

    Ok(format!(r"{win_dir}\{name}.png"))
}

fn resolve_icon(icon: &str) -> Result<String, String> {
    if icon.starts_with("http://") || icon.starts_with("https://") || icon.starts_with("file://") {
        return Ok(icon.to_string());
    }

    if BUILTIN_ICONS.iter().any(|(n, _)| *n == icon) {
        return extract_builtin_icon(icon);
    }

    let path = shellexpand_tilde(icon);
    if Path::new(&path).exists() {
        let abs = std::fs::canonicalize(&path).map_err(|e| {
            tf(
                "err.invalid_path",
                &[("path", &path), ("error", &e.to_string())],
            )
        })?;
        return wslpath_to_windows(&abs.to_string_lossy());
    }

    Ok(icon.to_string())
}

fn unescape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn shellexpand_tilde(input: &str) -> String {
    if let Some(rest) = input.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    if input == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return home;
        }
    }
    input.to_string()
}

fn notify(
    title: &str,
    message: &str,
    app_id: &str,
    icon: &str,
    footer: &str,
) -> Result<(), String> {
    let script = PS_TEMPLATE
        .replace("__TITLE__", title)
        .replace("__MESSAGE__", message)
        .replace("__ICON__", icon)
        .replace("__FOOTER__", footer)
        .replace("__APP_ID__", app_id);
    run_powershell(&script).map(|_| ())
}

fn settings_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".claude/settings.json")
}

fn install_command_string() -> Result<String, String> {
    let exe =
        std::env::current_exe().map_err(|e| tf("err.locate_exe", &[("error", &e.to_string())]))?;
    let exe_str = exe.to_string_lossy().into_owned();

    if let Ok(home) = std::env::var("HOME") {
        let canonical = format!("{home}/.claude/bin/wsl-claude-toast");
        if exe_str == canonical {
            return Ok("~/.claude/bin/wsl-claude-toast --hook".to_string());
        }
    }
    Ok(format!("{exe_str} --hook"))
}

fn hook_already_present(entries: &[Value], matcher: Option<&str>, command: &str) -> bool {
    entries.iter().any(|entry| {
        let entry_matcher = entry.get("matcher").and_then(Value::as_str);
        if entry_matcher != matcher {
            return false;
        }
        entry
            .get("hooks")
            .and_then(Value::as_array)
            .map(|hooks| {
                hooks.iter().any(|h| {
                    h.get("command").and_then(Value::as_str) == Some(command)
                        && h.get("type").and_then(Value::as_str) == Some("command")
                })
            })
            .unwrap_or(false)
    })
}

fn ensure_event_hook(
    hooks_root: &mut Value,
    event: &str,
    matcher: Option<&str>,
    command: &str,
) -> bool {
    let map = hooks_root.as_object_mut().expect("hooks must be an object");
    let entries = map
        .entry(event.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));

    let arr = match entries.as_array_mut() {
        Some(a) => a,
        None => {
            *entries = Value::Array(Vec::new());
            entries.as_array_mut().unwrap()
        }
    };

    if hook_already_present(arr, matcher, command) {
        return false;
    }

    let mut new_entry = serde_json::Map::new();
    if let Some(m) = matcher {
        new_entry.insert("matcher".to_string(), Value::String(m.to_string()));
    }
    new_entry.insert(
        "hooks".to_string(),
        json!([{ "type": "command", "command": command, "async": true }]),
    );
    arr.push(Value::Object(new_entry));
    true
}

fn install_hook() -> Result<(), String> {
    println!(
        "{}",
        tf("cli.appid.registering", &[("appId", PERSONAL_APP_ID)])
    );
    match register_personal_appid() {
        Ok(true) => println!(
            "{}",
            tf(
                "cli.appid.installed",
                &[("appId", PERSONAL_APP_ID), ("appName", PERSONAL_APP_NAME)]
            )
        ),
        Ok(false) => println!("{}", tf("cli.appid.already", &[("appId", PERSONAL_APP_ID)])),
        Err(e) => eprintln!("{}", tf("cli.appid.register_warn", &[("error", &e)])),
    }

    let path = settings_path();
    let path_str = path.display().to_string();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            tf(
                "err.settings.create_dir",
                &[
                    ("path", &parent.display().to_string()),
                    ("error", &e.to_string()),
                ],
            )
        })?;
    }

    let mut root: Value = if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| {
            tf(
                "err.settings.read",
                &[("path", &path_str), ("error", &e.to_string())],
            )
        })?;
        if content.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&content).map_err(|e| {
                tf(
                    "err.settings.invalid_json",
                    &[("path", &path_str), ("error", &e.to_string())],
                )
            })?
        }
    } else {
        json!({})
    };

    if !root.is_object() {
        return Err(tf("err.settings.not_object", &[("path", &path_str)]));
    }

    let command = install_command_string()?;
    let hooks_root = root
        .as_object_mut()
        .unwrap()
        .entry("hooks".to_string())
        .or_insert_with(|| json!({}));
    if !hooks_root.is_object() {
        return Err(t("err.hooks.not_object").to_string());
    }

    let added_stop = ensure_event_hook(hooks_root, "Stop", None, &command);
    let added_notif = ensure_event_hook(
        hooks_root,
        "Notification",
        Some("permission_prompt"),
        &command,
    );

    if !added_stop && !added_notif {
        println!("{}", tf("cli.hook.already", &[("path", &path_str)]));
        return Ok(());
    }

    let serialized = serde_json::to_string_pretty(&root)
        .map_err(|e| tf("err.settings.serialize", &[("error", &e.to_string())]))?;
    std::fs::write(&path, format!("{serialized}\n")).map_err(|e| {
        tf(
            "err.settings.write",
            &[("path", &path_str), ("error", &e.to_string())],
        )
    })?;

    let mut events: Vec<String> = Vec::new();
    if added_stop {
        events.push("Stop".to_string());
    }
    if added_notif {
        events.push(t("cli.event.notification").to_string());
    }
    println!(
        "{}",
        tf(
            "cli.hook.installed",
            &[("events", &events.join(", ")), ("command", &command)]
        )
    );
    Ok(())
}

fn candidate_commands() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        out.push(format!("{} --hook", exe.to_string_lossy()));
    }
    if let Ok(home) = std::env::var("HOME") {
        out.push(format!("{home}/.claude/bin/wsl-claude-toast --hook"));
        out.push(format!("{home}/.claude/bin/notify --hook"));
    }
    out.push("~/.claude/bin/wsl-claude-toast --hook".to_string());
    out.push("~/.claude/bin/notify --hook".to_string());
    out
}

fn uninstall_hook() -> Result<(), String> {
    let path = settings_path();
    let path_str = path.display().to_string();
    if !path.exists() {
        println!("{}", tf("cli.no_settings_file", &[("path", &path_str)]));
        return Ok(());
    }

    let content = std::fs::read_to_string(&path).map_err(|e| {
        tf(
            "err.settings.read",
            &[("path", &path_str), ("error", &e.to_string())],
        )
    })?;
    let mut root: Value = serde_json::from_str(&content).map_err(|e| {
        tf(
            "err.settings.invalid_json",
            &[("path", &path_str), ("error", &e.to_string())],
        )
    })?;

    let candidates = candidate_commands();
    let is_ours = |cmd: &str| candidates.iter().any(|c| c == cmd);

    let mut removed: Vec<String> = Vec::new();

    let hooks_root = root
        .as_object_mut()
        .and_then(|o| o.get_mut("hooks"))
        .and_then(Value::as_object_mut);

    if let Some(hooks_map) = hooks_root {
        for event in ["Stop", "Notification"] {
            let Some(arr) = hooks_map.get_mut(event).and_then(Value::as_array_mut) else {
                continue;
            };

            let before = arr.len();
            arr.retain_mut(|entry| {
                let Some(inner) = entry.get_mut("hooks").and_then(Value::as_array_mut) else {
                    return true;
                };
                inner.retain(|h| {
                    h.get("command")
                        .and_then(Value::as_str)
                        .map(|c| !is_ours(c))
                        .unwrap_or(true)
                });
                !inner.is_empty()
            });

            if arr.len() != before {
                removed.push(event.to_string());
            }
            if arr.is_empty() {
                hooks_map.remove(event);
            }
        }

        if hooks_map.is_empty() {
            root.as_object_mut().unwrap().remove("hooks");
        }
    }

    if !removed.is_empty() {
        let serialized = serde_json::to_string_pretty(&root)
            .map_err(|e| tf("err.settings.serialize", &[("error", &e.to_string())]))?;
        std::fs::write(&path, format!("{serialized}\n")).map_err(|e| {
            tf(
                "err.settings.write",
                &[("path", &path_str), ("error", &e.to_string())],
            )
        })?;
        println!(
            "{}",
            tf("cli.hook.removed", &[("events", &removed.join(", "))])
        );
    } else {
        println!("{}", tf("cli.hook.absent", &[("path", &path_str)]));
    }

    match unregister_personal_appid() {
        Ok(true) => println!(
            "{}",
            tf("cli.lnk.removed", &[("appName", PERSONAL_APP_NAME)])
        ),
        Ok(false) => println!("{}", t("cli.lnk.absent")),
        Err(e) => eprintln!("{}", tf("cli.lnk.remove_warn", &[("error", &e)])),
    }

    Ok(())
}

fn die(msg: impl AsRef<str>) -> ! {
    eprintln!("{} {}", t("err.prefix"), msg.as_ref());
    std::process::exit(1);
}

fn main() {
    let args = Args::parse();

    if args.install_hook {
        if let Err(e) = install_hook() {
            die(e);
        }
        return;
    }

    if args.uninstall_hook {
        if let Err(e) = uninstall_hook() {
            die(e);
        }
        return;
    }

    let (raw_title, raw_message, raw_icon, raw_footer) = if args.hook {
        let payload = parse_hook_input().unwrap_or_else(|e| die(e));
        (
            payload.title,
            payload.message,
            args.icon.clone().unwrap_or(payload.icon),
            args.footer.clone().unwrap_or(payload.footer),
        )
    } else {
        let title = args
            .title
            .clone()
            .unwrap_or_else(|| die(t("err.title_required")));
        let message = args
            .message
            .clone()
            .unwrap_or_else(|| die(t("err.message_required")));
        (
            title,
            message,
            args.icon.clone().unwrap_or_default(),
            args.footer.clone().unwrap_or_default(),
        )
    };

    let app_id = match args.app_id {
        Some(id) => id,
        None => detect_app_id().unwrap_or_else(|e| die(e)),
    };

    let icon = if raw_icon.is_empty() {
        String::new()
    } else {
        resolve_icon(&raw_icon).unwrap_or_else(|e| die(e))
    };

    let title = unescape(&raw_title);
    let message = unescape(&raw_message);
    let footer = unescape(&raw_footer);

    if let Err(e) = notify(&title, &message, &app_id, &icon, &footer) {
        die(e);
    }
}
