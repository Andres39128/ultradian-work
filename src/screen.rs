use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BRIGHTNESS_SAVE_PATH: &str = "/tmp/ultradian-work-brightness";

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
        eprintln!("[ultradian-work] Failed to install Ctrl+C handler: {}", e);
    });
}

/// Dim screen during rest.
///
/// Attempts to reduce brightness via `brightnessctl`, saves the original
/// level to a temp file. Falls back to `xset dpms force off` if
/// brightnessctl is not available. Gracefully degrades with eprintln.
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
        let current = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Idempotent: if a level is already saved we are mid-rest; re-reading
        // would store the dimmed level as the "original" and break the restore.
        if !current.is_empty() && !PathBuf::from(BRIGHTNESS_SAVE_PATH).exists() {
            let _ = fs::write(BRIGHTNESS_SAVE_PATH, &current);
        }
        // Dim to 5%.
        let _ = Command::new("brightnessctl")
            .arg("s")
            .arg("5%")
            .arg("-n")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
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

    eprintln!("[ultradian-work] screen::dim_screen: neither brightnessctl nor xset is available, screen dimming skipped");
}

/// Restore screen brightness to the saved level.
///
/// If a saved brightness file exists, restores via `brightnessctl`.
/// Otherwise tries `xset dpms force on`. Gracefully degrades with eprintln.
pub fn restore_screen() {
    if cfg!(test) {
        return;
    }
    let save_path = PathBuf::from(BRIGHTNESS_SAVE_PATH);

    if save_path.exists() {
        if let Ok(level) = fs::read_to_string(&save_path) {
            let level = level.trim();
            if !level.is_empty() {
                let _ = Command::new("brightnessctl")
                    .arg("s")
                    .arg(level)
                    .arg("-n")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
            }
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

    eprintln!("[ultradian-work] screen::restore_screen: could not restore screen state");
}

/// Lock the system session.
///
/// Tries `loginctl lock-session` first, falls back to `xdg-screensaver lock`.
/// Gracefully degrades with eprintln if neither is available.
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

    eprintln!("[ultradian-work] screen::lock_screen: neither loginctl nor xdg-screensaver is available, screen locking skipped");
}

/// Unlock the system session.
///
/// Tries `loginctl unlock-session` first, falls back to `xdg-screensaver unlock`.
/// Gracefully degrades with eprintln if neither is available.
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

    eprintln!("[ultradian-work] screen::unlock_screen: neither loginctl nor xdg-screensaver is available, screen unlocking skipped");
}

/// Get the saved brightness value from the temp file, if it exists.
#[allow(dead_code)]
pub fn get_saved_brightness() -> Option<String> {
    get_saved_brightness_from(&PathBuf::from(BRIGHTNESS_SAVE_PATH))
}

/// Persist a brightness level to the temp file so it can be restored later.
#[allow(dead_code)]
pub fn save_brightness(level: &str) {
    save_brightness_to(&PathBuf::from(BRIGHTNESS_SAVE_PATH), level);
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

fn save_brightness_to(path: &std::path::Path, level: &str) {
    if !level.is_empty() {
        let _ = fs::write(path, level);
    }
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
        save_brightness_to(&temp, "100");
        let saved = get_saved_brightness_from(&temp);
        assert_eq!(saved, Some("100".to_string()));

        // Saving empty string must not write.
        let _ = std::fs::remove_file(&temp);
        save_brightness_to(&temp, "");
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
}
