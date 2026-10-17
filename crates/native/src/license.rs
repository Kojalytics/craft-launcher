//! This machine's Kojalytics license: its device hash, the saved activation certificate, and
//! activating a key at kojalytics.com.

use artcraft_launcher_core::license::{self, License};
use sha2::{Digest, Sha256};

use crate::http::Http;
use crate::paths::Paths;

/// A stable, anonymous id for this machine: SHA-256 of the hardware/OS machine id. `None` when
/// the system has none we can read.
pub fn device_hash() -> Option<String> {
    let id = machine_id()?;
    Some(crate::http::hex(&Sha256::digest(format!("kojalytics-device:{id}").as_bytes())))
}

#[cfg(target_os = "macos")]
fn machine_id() -> Option<String> {
    let out = std::process::Command::new("/usr/sbin/ioreg").args(["-rd1", "-c", "IOPlatformExpertDevice"]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.contains("\"IOPlatformUUID\""))?;
    let id = line.rsplit('"').nth(1)?.trim().to_string();
    (!id.is_empty()).then_some(id)
}

#[cfg(target_os = "windows")]
fn machine_id() -> Option<String> {
    let out = std::process::Command::new("reg").args(["query", r"HKLM\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let id = text.lines().find(|l| l.contains("MachineGuid"))?.split_whitespace().last()?.to_string();
    (!id.is_empty()).then_some(id)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn machine_id() -> Option<String> {
    ["/etc/machine-id", "/var/lib/dbus/machine-id", "/etc/hostid"]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
}

fn certificate_path(paths: &Paths) -> std::path::PathBuf {
    paths.data.join("license.txt")
}

/// The saved license, if there is one and it is valid for this machine.
pub fn load(paths: &Paths, device: &str) -> Option<License> {
    let cert = std::fs::read_to_string(certificate_path(paths)).ok()?;
    match license::verify(&cert, device) {
        Ok(l) => Some(l),
        Err(e) => {
            log::warn!("saved license ignored: {e}");
            None
        }
    }
}

/// Forgets this machine's license.
pub fn remove(paths: &Paths) {
    let _ = std::fs::remove_file(certificate_path(paths));
}

/// Activates `key` for this machine and saves the certificate. The error is a sentence for the user.
pub fn activate(http: &Http, paths: &Paths, key: &str, device: &str) -> Result<License, String> {
    let Some(body) = license::normalize_key(key) else {
        return Err("That isn't a valid license key. It looks like KOJA-XXXXX-XXXXX-XXXXX-XXXXX.".into());
    };
    let url = std::env::var("KOJALYTICS_LICENSE_URL").unwrap_or_else(|_| license::ACTIVATE_URL.to_string());
    let request = serde_json::json!({
        "key": body,
        "device": device,
        "deviceName": hostname(),
        "app": "launcher",
        "version": env!("CARGO_PKG_VERSION"),
    });
    let (status, text) = http.post_json(&url, &request.to_string())?;
    let answer: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    if status != 200 {
        return Err(answer["error"].as_str().map(str::to_string).unwrap_or_else(|| format!("The license server answered HTTP {status}.")));
    }
    let cert = answer["certificate"].as_str().ok_or("The license server sent an unreadable answer.")?;
    let license = license::verify(cert, device).map_err(|e| format!("The license server sent an invalid certificate ({e})."))?;
    crate::paths::write_atomic(&certificate_path(paths), cert.as_bytes())?;
    Ok(license)
}

fn hostname() -> Option<String> {
    let out = std::process::Command::new("hostname").output().ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_machine_has_a_device_hash() {
        let hash = device_hash().expect("a machine id");
        assert_eq!(hash.len(), 64);
        assert_eq!(device_hash().unwrap(), hash, "stable");
    }

    #[test]
    fn a_missing_or_foreign_certificate_is_no_license() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        assert_eq!(load(&paths, "dev"), None);
        std::fs::create_dir_all(&paths.data).unwrap();
        std::fs::write(certificate_path(&paths), "eyJ2IjoxfQ.AAAA").unwrap();
        assert_eq!(load(&paths, "dev"), None);
        remove(&paths);
        assert!(!certificate_path(&paths).exists());
    }
}
