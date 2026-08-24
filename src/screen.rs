use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Where the pre-rest brightness level is saved, next to `tracker_data.json`
/// (e.g. `~/.local/share/com.DevPersonal.UltradianTimer/brightness`).
pub fn brightness_save_path() -> PathBuf {
    crate::tracker::TimeTrackerState::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("brightness")
}

/// Parses the first brightness level from `brightnessctl g` output.
///
/// With multiple monitors brightnessctl prints one line per device
/// (e.g. `74% [backlight]` then `45% [led]`); the level is taken from the
/// first non-empty line, stripping the device-class annotation. `brightnessctl s`
/// without `-d` targets the same default device.
pub(crate) fn parse_brightness_output(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .map(|line| line.trim())
        .find(|line| !line.is_empty())
        .map(|line| {
            line.split_once('[')
                .map(|(level, _)| level.trim())
                .unwrap_or(line)
                .to_string()
        })
        .filter(|level| !level.is_empty())
}

#[allow(dead_code)]
pub struct ScreenTools {
    has_brightnessctl: bool,
    has_xset: bool,
    has_lock: bool,
}

#[allow(dead_code)]
impl ScreenTools {
    pub fn detect() -> Self {
        Self {
            has_brightnessctl: Self::tool_exists("brightnessctl"),
            has_xset: Self::tool_exists("xset"),
            has_lock: Self::lock_tool_available(),
        }
    }

    pub fn has_brightnessctl(&self) -> bool {
        self.has_brightnessctl
    }

    pub fn has_xset(&self) -> bool {
        self.has_xset
    }

    pub fn has_lock(&self) -> bool {
        self.has_lock
    }

    pub fn has_dim_support(&self) -> bool {
        self.has_brightnessctl || self.has_xset
    }

    fn tool_exists(cmd: &str) -> bool {
        Command::new(cmd)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn lock_tool_available() -> bool {
        Command::new("loginctl")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            || Command::new("xdg-screensaver")
                .arg("--help")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
    }
}

pub fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").map(|v| v == "wayland").unwrap_or(false)
        || std::env::var("WAYLAND_DISPLAY").is_ok()
}

pub fn install_signal_handlers() {
    ctrlc::set_handler(|| {
        restore_screen();
        unlock_screen();
        std::process::exit(0);
    }).unwrap_or_else(|e| {
        tracing::error!(error = %e, "failed to install Ctrl+C handler");
    });
}

/// Dim screen during rest.
///
/// Attempts to reduce brightness via `brightnessctl`, saves the original
/// level to the project data dir. Falls back to `xset dpms force off` if
/// brightnessctl is not available. Gracefully degrades with tracing.
pub fn dim_screen() {
    if cfg!(test) {
        return;
    }
    // Try brightnessctl first: get current level, save it, then dim to 5%.
    if let Ok(output) = Command::new("brightnessctl")
        .arg("g")
        .output()
        && output.status.success()
    {
        let current = String::from_utf8_lossy(&output.stdout);
        let save_path = brightness_save_path();
        // Idempotent: if a level is already saved we are mid-rest; re-reading
        // would store the dimmed level as the "original" and break the restore.
        if !save_path.exists()
            && let Some(level) = parse_brightness_output(&current)
            && let Err(e) = save_brightness_to(&save_path, &level) {
            tracing::error!(error = %e, "failed to save current brightness level, restore after rest will be skipped");
        }
        // Dim to 5%.
        if let Err(e) = Command::new("brightnessctl")
            .arg("s")
            .arg("5%")
            .arg("-n")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
        {
            tracing::warn!(error = %e, "failed to dim screen via brightnessctl");
        }
        return;
    }

    // Fallback: xset dpms force off.
    if let Ok(status) = Command::new("xset")
        .arg("dpms")
        .arg("force")
        .arg("off")
        .status()
        && status.success()
    {
        return;
    }

    tracing::warn!("neither brightnessctl nor xset is available, screen dimming skipped");
}

/// Restore screen brightness to the saved level.
///
/// If a saved brightness file exists, restores via `brightnessctl`.
/// Otherwise tries `xset dpms force on`. Gracefully degrades with tracing.
pub fn restore_screen() {
    if cfg!(test) {
        return;
    }
    let save_path = brightness_save_path();

    if save_path.exists() {
        if let Some(level) = get_saved_brightness_from(&save_path)
            && let Err(e) = Command::new("brightnessctl")
                .arg("s")
                .arg(level)
                .arg("-n")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
        {
            tracing::warn!(error = %e, "failed to restore brightness via brightnessctl");
        }
        let _ = fs::remove_file(&save_path);
        return;
    }

    // Try xset dpms force on.
    if let Ok(status) = Command::new("xset")
        .arg("dpms")
        .arg("force")
        .arg("on")
        .status()
        && status.success()
    {
        return;
    }

    tracing::warn!("could not restore screen state");
}

/// Lock the system session.
///
/// Tries `loginctl lock-session` first, falls back to `xdg-screensaver lock`.
/// Gracefully degrades with tracing if neither is available.
pub fn lock_screen() {
    if cfg!(test) {
        return;
    }
    if let Ok(status) = Command::new("loginctl")
        .arg("lock-session")
        .status()
        && status.success()
    {
        return;
    }

    if let Ok(status) = Command::new("xdg-screensaver")
        .arg("lock")
        .status()
        && status.success()
    {
        return;
    }

    tracing::warn!("neither loginctl nor xdg-screensaver is available, screen locking skipped");
}

/// Unlock the system session.
///
/// Tries `loginctl unlock-session` first, falls back to `xdg-screensaver unlock`.
/// Gracefully degrades with tracing if neither is available.
pub fn unlock_screen() {
    if cfg!(test) {
        return;
    }
    if let Ok(status) = Command::new("loginctl")
        .arg("unlock-session")
        .status()
        && status.success()
    {
        return;
    }

    if let Ok(status) = Command::new("xdg-screensaver")
        .arg("unlock")
        .status()
        && status.success()
    {
        return;
    }

    tracing::warn!("neither loginctl nor xdg-screensaver is available, screen unlocking skipped");
}

fn get_saved_brightness_from(path: &std::path::Path) -> Option<String> {
    if path.exists() {
        fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    }
}

fn save_brightness_to(path: &std::path::Path, level: &str) -> std::io::Result<()> {
    if level.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_does_not_panic() {
        let tools = ScreenTools::detect();
        // Detection should succeed (return a struct) regardless of tool
        // availability — missing tools just yield false booleans.
        let _ = tools.has_brightnessctl();
        let _ = tools.has_xset();
        let _ = tools.has_lock();
    }

    #[test]
    fn test_save_and_get_brightness_roundtrip() {
        let temp = std::env::temp_dir().join(format!("ultradian-test-save-{}", std::process::id()));
        let _ = std::fs::remove_file(&temp);

        // save + get roundtrip.
        save_brightness_to(&temp, "100").expect("save should succeed");
        let saved = get_saved_brightness_from(&temp);
        assert_eq!(saved, Some("100".to_string()));

        // Saving empty string must not write.
        let _ = std::fs::remove_file(&temp);
        save_brightness_to(&temp, "").expect("empty save should not fail");
        assert!(!temp.exists());

        // Clean up.
        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_get_brightness_when_no_file() {
        let temp = std::env::temp_dir().join(format!("ultradian-test-missing-{}", std::process::id()));
        let _ = std::fs::remove_file(&temp);
        assert_eq!(get_saved_brightness_from(&temp), None);
    }

#[test]
    fn test_dim_and_restore_do_not_panic() {
        dim_screen();
        restore_screen();
        lock_screen();
    }

    #[test]
    fn test_is_wayland_does_not_panic() {
        let _ = is_wayland();
    }

    #[test]
    fn test_detect_has_methods() {
        let tools = ScreenTools::detect();
        let _ = tools.has_dim_support();
    }

    #[test]
    fn test_parse_brightness_output_single_line() {
        assert_eq!(parse_brightness_output("74%"), Some("74%".to_string()));
    }

    #[test]
    fn test_parse_brightness_output_strips_device_annotation() {
        assert_eq!(parse_brightness_output("74% [unknown]"), Some("74%".to_string()));
    }

    #[test]
    fn test_parse_brightness_output_multi_monitor_takes_first_line() {
        let stdout = "74% [backlight]\n45% [led]\n";
        assert_eq!(parse_brightness_output(stdout), Some("74%".to_string()));
    }

    #[test]
    fn test_parse_brightness_output_skips_blank_lines() {
        assert_eq!(parse_brightness_output("\n\n  80% [backlight]\n"), Some("80%".to_string()));
    }

    #[test]
    fn test_parse_brightness_output_empty() {
        assert_eq!(parse_brightness_output(""), None);
        assert_eq!(parse_brightness_output("   \n  \n"), None);
        assert_eq!(parse_brightness_output("[backlight]\n"), None);
    }
}
