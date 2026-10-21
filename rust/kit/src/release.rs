//! Preview downloads trust the fixed GitHub HTTPS endpoints. No manifests or signing.
use std::{
    error::Error,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::NamedTempFile;

use crate::MacosPayload;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const RELEASE_BASE: &str = "https://github.com/ahappypie/builderkit/releases/download";
pub const NIX_INSTALLER_VERSION: &str = "2.35.3";
pub const NIX_INSTALLER_URL: &str =
    "https://github.com/NixOS/nix-installer/releases/download/2.35.3/nix-installer-aarch64-darwin";
const FILE_LIMIT: u64 = 16 * 1024 * 1024;
const DOWNLOAD_LIMIT: u64 = 256 * 1024 * 1024;

pub fn validate_version(value: &str) -> std::result::Result<String, String> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts[0].len() != 4
        || parts[1].len() != 2
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
        || !matches!(parts[1].parse::<u32>(), Ok(1..=12))
        || parts.iter().any(|part| part.parse::<u32>().is_err())
    {
        return Err("expected an explicit release version: YYYY.MM.PATCH (not latest)".to_owned());
    }
    Ok(value.to_owned())
}

pub fn configuration_url(version: &str) -> Result<String> {
    validate_version(version)?;
    // Encode the slash in the Git tag while keeping the asset filename unprefixed.
    Ok(format!(
        "{RELEASE_BASE}/bundle%2F{version}/builderkit-darwin-{version}.tar.gz"
    ))
}

fn curl(url: &str) -> Result<Command> {
    if !url.starts_with("https://") {
        return Err("release downloads require HTTPS".into());
    }
    let mut command = Command::new("curl");
    command.args([
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "30",
        "--max-time",
        "600",
    ]);
    command.arg("--url").arg(url);
    Ok(command)
}

pub fn check_available(version: &str) -> Result<()> {
    let url = configuration_url(version)?;
    let status = curl(&url)?
        .args(["--head", "--output", "/dev/null"])
        .status()?;
    if !status.success() {
        return Err(format!("requested release is unavailable: {url} ({status})").into());
    }
    Ok(())
}

fn download(url: &str, destination: &Path) -> Result<()> {
    let status = curl(url)?
        .arg("--max-filesize")
        .arg(DOWNLOAD_LIMIT.to_string())
        .arg("--output")
        .arg(destination)
        .status()?;
    if !status.success() {
        return Err(format!("download failed: {url} ({status})").into());
    }
    let size = fs::metadata(destination)?.len();
    if size == 0 || size > DOWNLOAD_LIMIT {
        return Err("download is empty or exceeds the preview size limit".into());
    }
    Ok(())
}

pub fn prepare_payload(root: &Path, version: &str) -> Result<MacosPayload> {
    prepare_payload_with(root, version, download)
}

fn prepare_payload_with(
    root: &Path,
    version: &str,
    fetch: impl FnOnce(&str, &Path) -> Result<()>,
) -> Result<MacosPayload> {
    let url = configuration_url(version)?;
    let directory = root.join("releases").join(version);
    // Stage on the same filesystem, without creating a release directory until
    // both the download and archive validation succeed.
    fs::create_dir_all(root)?;
    let temporary = NamedTempFile::new_in(root)?;
    println!("Downloading {url}");
    fetch(&url, temporary.path())?;
    // Never extract arbitrary archive paths onto the filesystem.
    let payload = read_payload(temporary.path())?;
    fs::create_dir_all(&directory)?;
    temporary.persist(directory.join("darwin.tar.gz"))?;
    Ok(payload)
}

pub fn prepare_installer(directory: &Path) -> Result<PathBuf> {
    fs::create_dir_all(directory)?;
    let temporary = NamedTempFile::new_in(directory)?;
    println!("Downloading {NIX_INSTALLER_URL}");
    download(NIX_INSTALLER_URL, temporary.path())?;
    let destination = directory.join(format!(
        "nix-installer-{NIX_INSTALLER_VERSION}-aarch64-darwin"
    ));
    temporary.persist(&destination)?;
    Ok(destination)
}

fn read_payload(path: &Path) -> Result<MacosPayload> {
    let decoder =
        flate2::read::GzDecoder::new(fs::File::open(path)?).take(2 * FILE_LIMIT + 1024 * 1024);
    let mut archive = tar::Archive::new(decoder);
    let mut flake = None;
    let mut lock = None;
    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() || entry.size() > FILE_LIMIT {
            return Err("configuration archive must contain only bounded regular files".into());
        }
        let name = entry.path()?.into_owned();
        let slot = match name.to_str() {
            Some("flake.nix" | "./flake.nix") => &mut flake,
            Some("flake.lock" | "./flake.lock") => &mut lock,
            _ => return Err("unexpected file in configuration archive".into()),
        };
        let mut value = String::new();
        entry.read_to_string(&mut value)?;
        if slot.replace(value).is_some() {
            return Err("duplicate file in configuration archive".into());
        }
    }
    // Read through the gzip trailer too, detecting truncated/corrupt transfers even
    // when tar has already reached its end markers. Bound total decompression.
    let mut decoder = archive.into_inner();
    std::io::copy(&mut decoder, &mut std::io::sink())?;
    if decoder.limit() == 0 {
        return Err("configuration archive exceeds its decompressed size limit".into());
    }
    let payload = MacosPayload {
        flake: flake.ok_or("configuration archive is missing flake.nix")?,
        lock: lock.ok_or("configuration archive is missing flake.lock")?,
    };
    payload.validate()?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive_bytes(entries: &[(&str, tar::EntryType, &[u8])]) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        for (name, kind, contents) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(*kind);
            header.set_mode(0o644);
            header.set_size(contents.len() as u64);
            if kind.is_symlink() {
                header.set_link_name("/tmp/untrusted").unwrap();
            }
            header.set_cksum();
            archive.append_data(&mut header, name, *contents).unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap()
    }

    fn valid_archive() -> Vec<u8> {
        archive_bytes(&[
            ("flake.nix", tar::EntryType::Regular,
                b"# BEGIN BUILDER-KIT USER SETTINGS\n# END BUILDER-KIT USER SETTINGS\n# BEGIN BUILDER-KIT MANAGED IMPLEMENTATION"),
            ("flake.lock", tar::EntryType::Regular, b"{}"),
        ])
    }

    #[test]
    fn versions_are_explicit_and_safe_for_urls_and_paths() {
        for value in [
            "latest",
            "",
            "../2026.09.0",
            "bundle/2026.09.0",
            "kit/0.0.2",
            "2026.09.0?x",
            "2026.13.0",
            "2026.9.0",
            "2026.09.0-edge",
            "2026.09.+1",
        ] {
            assert!(validate_version(value).is_err(), "{value}");
        }
        assert_eq!(
            configuration_url("2026.09.0").unwrap(),
            "https://github.com/ahappypie/builderkit/releases/download/bundle%2F2026.09.0/builderkit-darwin-2026.09.0.tar.gz"
        );
        assert!(curl("http://example.com/payload").is_err());
    }

    #[test]
    fn prepares_requested_release_without_installing() {
        let root = tempfile::tempdir().unwrap();
        prepare_payload_with(root.path(), "2026.09.0", |url, path| {
            assert!(url.ends_with("/bundle%2F2026.09.0/builderkit-darwin-2026.09.0.tar.gz"));
            fs::write(path, valid_archive())?;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            fs::read_dir(root.path().join("releases/2026.09.0"))
                .unwrap()
                .count(),
            1
        );
        assert!(!root.path().join("state").exists());
        assert!(!root.path().join("nix").exists());
        // Redownload rather than trusting unsigned cached bytes.
        let result = prepare_payload_with(root.path(), "2026.09.0", |_, path| {
            fs::write(path, b"partial")?;
            Err("connection interrupted".into())
        });
        assert!(result.is_err());
        assert_eq!(
            fs::read(root.path().join("releases/2026.09.0/darwin.tar.gz")).unwrap(),
            valid_archive()
        );
    }

    #[test]
    fn failed_download_does_not_publish_partial_files() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            prepare_payload_with(root.path(), "2026.09.0", |_, path| {
                fs::write(path, b"partial")?;
                Err("connection interrupted".into())
            })
            .is_err()
        );
        assert!(!root.path().join("releases").exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn invalid_archive_does_not_create_a_release_directory() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            prepare_payload_with(root.path(), "2026.09.0", |_, path| {
                fs::write(path, b"not an archive")?;
                Ok(())
            })
            .is_err()
        );
        assert!(!root.path().join("releases").exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn failed_download_leaves_other_releases_untouched() {
        let root = tempfile::tempdir().unwrap();
        let existing = root.path().join("releases/2026.08.0");
        fs::create_dir_all(&existing).unwrap();
        fs::write(existing.join("darwin.tar.gz"), b"existing download").unwrap();
        assert!(
            prepare_payload_with(root.path(), "2026.09.0", |_, _| {
                Err("release not found".into())
            })
            .is_err()
        );
        assert!(!root.path().join("releases/2026.09.0").exists());
        assert_eq!(
            fs::read(existing.join("darwin.tar.gz")).unwrap(),
            b"existing download"
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn rejects_unexpected_duplicate_missing_symlink_and_truncated_archives() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("payload.tar.gz");
        fs::write(&path, valid_archive()).unwrap();
        assert!(read_payload(&path).is_ok());
        for entries in [
            vec![("extra", tar::EntryType::Regular, b"abc".as_slice())],
            vec![("flake.nix", tar::EntryType::Symlink, b"".as_slice())],
            vec![("flake.nix", tar::EntryType::Regular, b"abc".as_slice())],
            vec![
                ("flake.nix", tar::EntryType::Regular, b"abc".as_slice()),
                ("flake.nix", tar::EntryType::Regular, b"abc".as_slice()),
            ],
        ] {
            fs::write(&path, archive_bytes(&entries)).unwrap();
            assert!(read_payload(&path).is_err());
        }
        let truncated = valid_archive();
        for cut in [truncated.len() / 2, truncated.len() - 4] {
            fs::write(&path, &truncated[..cut]).unwrap();
            assert!(read_payload(&path).is_err());
        }
    }
}
