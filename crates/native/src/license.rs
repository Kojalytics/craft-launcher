//! This machine's Kojalytics license: its device hash, the saved activation certificate, and
//! activating a key at kojalytics.com and renewing its certificate there. Also the free trial: its
//! certificate sits next to the license (`trial.txt`), where the apps look for it too.

use artcraft_launcher_core::license::{self, License, Trial};
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
    let out = crate::process::command("reg").args(["query", r"HKLM\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"]).output().ok()?;
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

/// The saved license, if there is one, it is valid for this machine and it hasn't run out.
pub fn load(paths: &Paths, device: &str) -> Option<License> {
    let license = saved(paths, device, &license::PUBLIC_KEY)?;
    if license.expired(crate::now()) {
        log::warn!("saved license ran out: it needs renewing online");
        return None;
    }
    Some(license)
}

/// The saved license if its certificate is valid for this machine, run out or not.
pub fn saved(paths: &Paths, device: &str, public_key: &[u8; 32]) -> Option<License> {
    let cert = std::fs::read_to_string(certificate_path(paths)).ok()?;
    match license::verify_with(&cert, device, public_key) {
        Ok(l) => Some(l),
        Err(e) => {
            log::warn!("saved license ignored: {e}");
            None
        }
    }
}

/// What kojalytics.com said when the launcher renewed its license certificate.
#[derive(Debug, PartialEq, Eq)]
pub enum Renewal {
    /// A fresh certificate, saved.
    Renewed(License),
    /// The key was deactivated or this computer removed from it: the certificate is deleted, so the
    /// apps lock. The text is the server's sentence for the user.
    Revoked(String),
    /// No answer (offline, server error): the certificate stays until it runs out.
    Failed(String),
}

/// Renews the saved license certificate at kojalytics.com. `None` when there is none for this
/// machine, or another activation replaced it meanwhile.
pub fn renew(http: &Http, paths: &Paths, device: &str) -> Option<Renewal> {
    let url = std::env::var("KOJALYTICS_REFRESH_URL").unwrap_or_else(|_| license::REFRESH_URL.to_string());
    renew_with(http, paths, device, &url, &license::PUBLIC_KEY)
}

fn renew_with(http: &Http, paths: &Paths, device: &str, url: &str, public_key: &[u8; 32]) -> Option<Renewal> {
    let path = certificate_path(paths);
    let cert = std::fs::read_to_string(&path).ok()?;
    license::verify_with(&cert, device, public_key).ok()?;
    let request = serde_json::json!({ "certificate": cert.trim(), "version": env!("CARGO_PKG_VERSION") });
    let (status, text) = match http.post_json(url, &request.to_string()) {
        Ok(answer) => answer,
        Err(e) => return Some(Renewal::Failed(e)),
    };
    let answer: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    let error = || answer["error"].as_str().map(str::to_string).unwrap_or_else(|| format!("The license server answered HTTP {status}."));
    // A key activated while the request was on its way is left alone.
    let unchanged = || std::fs::read_to_string(&path).is_ok_and(|now| now == cert);
    match status {
        200 => {
            let Some(fresh) = answer["certificate"].as_str() else {
                return Some(Renewal::Failed("The license server sent an unreadable answer.".into()));
            };
            let license = match license::verify_with(fresh, device, public_key) {
                Ok(l) => l,
                Err(e) => return Some(Renewal::Failed(format!("The license server sent an invalid certificate ({e})."))),
            };
            if !unchanged() {
                return None;
            }
            Some(match crate::paths::write_atomic(&path, fresh.as_bytes()) {
                Ok(()) => Renewal::Renewed(license),
                Err(e) => Renewal::Failed(e),
            })
        }
        403 => {
            if !unchanged() {
                return None;
            }
            remove(paths);
            Some(Renewal::Revoked(error()))
        }
        _ => Some(Renewal::Failed(error())),
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

fn trial_path(paths: &Paths) -> std::path::PathBuf {
    paths.data.join("trial.txt")
}

/// This machine's saved trial, running or over, if its certificate is valid for this machine.
pub fn load_trial(paths: &Paths, device: &str) -> Option<Trial> {
    let cert = std::fs::read_to_string(trial_path(paths)).ok()?;
    match license::verify_trial(&cert, device) {
        Ok(t) => Some(t),
        Err(e) => {
            log::warn!("saved trial ignored: {e}");
            None
        }
    }
}

/// Starts this machine's free trial at kojalytics.com (or fetches its certificate again) and saves
/// the certificate. An ended trial is saved too, so the apps can say it ended. The error is a
/// sentence for the user.
pub fn start_trial(http: &Http, paths: &Paths, device: &str) -> Result<Trial, String> {
    let url = std::env::var("KOJALYTICS_TRIAL_URL").unwrap_or_else(|_| license::TRIAL_URL.to_string());
    let request = serde_json::json!({
        "device": device,
        "deviceName": hostname(),
        "app": "launcher",
        "version": env!("CARGO_PKG_VERSION"),
    });
    let (status, text) = http.post_json(&url, &request.to_string())?;
    let answer: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    if let Some(cert) = answer["certificate"].as_str() {
        let trial = license::verify_trial(cert, device).map_err(|e| format!("The license server sent an invalid certificate ({e})."))?;
        crate::paths::write_atomic(&trial_path(paths), cert.as_bytes())?;
        if status == 200 {
            return Ok(trial);
        }
    }
    Err(answer["error"].as_str().map(str::to_string).unwrap_or_else(|| format!("The license server answered HTTP {status}.")))
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

    /// What a customer's launcher does on "Start free 3-day trial", against kojalytics.com. It uses up
    /// the machine's one trial, so it runs only on throwaway machines (kojalytics-windows.yml, e2e).
    #[test]
    #[ignore = "starts this machine's one free trial at kojalytics.com: run it on a throwaway machine"]
    fn starts_a_free_trial_against_the_live_server() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        let device = device_hash().unwrap();
        let trial = start_trial(&Http::new().unwrap(), &paths, &device).unwrap();
        assert!(trial.expires > crate::now(), "{trial:?}");
        assert_eq!(trial.expires - trial.starts, 3 * 24 * 60 * 60);
        assert_eq!(load_trial(&paths, &device), Some(trial));
    }

    /// `KOJALYTICS_LICENSE_URL=http://localhost:3100/api/license/activate KOJA_TEST_KEY=KOJA-… cargo test -- --ignored`
    #[test]
    #[ignore = "needs a license server (KOJALYTICS_LICENSE_URL) and a fresh key (KOJA_TEST_KEY)"]
    fn activates_a_key_against_a_live_server() {
        let key = std::env::var("KOJA_TEST_KEY").expect("KOJA_TEST_KEY");
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        let device = device_hash().unwrap();
        let license = activate(&Http::new().unwrap(), &paths, &key, &device).unwrap();
        assert_eq!(license.hint, license::normalize_key(&key).unwrap()[15..]);
        assert_eq!(load(&paths, &device), Some(license));
        // Activating again on the same device doesn't use up another seat.
        activate(&Http::new().unwrap(), &paths, &key, &device).unwrap();
    }

    use std::sync::{Arc, Mutex};

    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use ed25519_dalek::{Signer, SigningKey};

    fn signer() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    fn cert(device: &str, issued: i64, expires: Option<i64>) -> String {
        let expires = expires.map(|e| format!(r#","expires":{e}"#)).unwrap_or_default();
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"v":1,"license":"abc","hint":"STVWX","device":"{device}","issued":{issued}{expires}}}"#));
        format!("{payload}.{}", URL_SAFE_NO_PAD.encode(signer().sign(payload.as_bytes()).to_bytes()))
    }

    /// A license server answering every request with `status` and `body`; returns its URL and the
    /// request bodies it got.
    fn server(status: &'static str, body: String) -> (String, Arc<Mutex<Vec<String>>>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/api/license/refresh", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = Vec::new();
                let mut buf = [0u8; 4096];
                // Headers, then as much body as Content-Length says.
                while let Ok(n) = stream.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&request);
                    if let Some((head, rest)) = text.split_once("\r\n\r\n") {
                        let length = head
                            .lines()
                            .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                            .unwrap_or(0);
                        if rest.len() >= length {
                            seen.lock().unwrap().push(rest.to_string());
                            break;
                        }
                    }
                }
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        (url, requests)
    }

    fn saved_dir(certificate: &str) -> (tempfile::TempDir, Paths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::under(dir.path());
        crate::paths::write_atomic(&certificate_path(&paths), certificate.as_bytes()).unwrap();
        (dir, paths)
    }

    fn renew_at(url: &str, paths: &Paths) -> Option<Renewal> {
        renew_with(&Http::new().unwrap(), paths, "dev1", url, &signer().verifying_key().to_bytes())
    }

    #[test]
    fn a_valid_key_gets_a_fresh_certificate_saved() {
        let old = cert("dev1", 1000, None);
        let fresh = cert("dev1", 2000, Some(2000 + 30 * 86_400));
        let (url, requests) = server("200 OK", serde_json::json!({ "certificate": fresh, "expires": 2000 + 30 * 86_400 }).to_string());
        let (_dir, paths) = saved_dir(&old);
        let Some(Renewal::Renewed(license)) = renew_at(&url, &paths) else { panic!("not renewed") };
        assert_eq!((license.issued, license.expires), (2000, Some(2000 + 30 * 86_400)));
        assert_eq!(std::fs::read_to_string(certificate_path(&paths)).unwrap(), fresh);
        // The server got the old certificate to renew.
        let sent: serde_json::Value = serde_json::from_str(&requests.lock().unwrap()[0]).unwrap();
        assert_eq!(sent["certificate"], old);
    }

    #[test]
    fn a_deactivated_key_loses_its_certificate() {
        let (url, _) = server("403 Forbidden", r#"{"error":"Dieser Lizenzschlüssel wurde deaktiviert."}"#.into());
        let (_dir, paths) = saved_dir(&cert("dev1", 1000, Some(5000)));
        assert_eq!(renew_at(&url, &paths), Some(Renewal::Revoked("Dieser Lizenzschlüssel wurde deaktiviert.".into())));
        assert!(!certificate_path(&paths).exists());
    }

    #[test]
    fn without_an_answer_the_certificate_stays() {
        let saved = cert("dev1", 1000, Some(5000));
        for (status, body) in [("500 Internal Server Error", r#"{"error":"License server error."}"#), ("200 OK", r#"{"certificate":"garbage"}"#)] {
            let (url, _) = server(status, body.into());
            let (_dir, paths) = saved_dir(&saved);
            assert!(matches!(renew_at(&url, &paths), Some(Renewal::Failed(_))), "{status}");
            assert_eq!(std::fs::read_to_string(certificate_path(&paths)).unwrap(), saved);
        }
        // Nobody listening.
        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/api/license/refresh", closed.local_addr().unwrap());
        drop(closed);
        let (_dir, paths) = saved_dir(&saved);
        assert!(matches!(renew_at(&url, &paths), Some(Renewal::Failed(_))));
        assert!(certificate_path(&paths).exists());
    }

    #[test]
    fn only_this_machines_certificate_is_renewed() {
        let (url, requests) = server("403 Forbidden", "{}".into());
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(renew_at(&url, &Paths::under(dir.path())), None);
        let (_dir, paths) = saved_dir(&cert("dev2", 1000, None));
        assert_eq!(renew_at(&url, &paths), None);
        assert!(certificate_path(&paths).exists());
        assert!(requests.lock().unwrap().is_empty());
    }

    #[test]
    fn a_run_out_certificate_is_still_read_for_renewing() {
        let public = signer().verifying_key().to_bytes();
        let (_dir, paths) = saved_dir(&cert("dev1", 1000, Some(5000)));
        assert!(saved(&paths, "dev1", &public).is_some_and(|l| l.expired(crate::now())));
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
