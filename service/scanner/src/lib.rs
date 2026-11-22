//! Scanner-Orchestrierung: Defender-Urteil und Dateityp-Regeln.
//!
//! Die Tauri-Shell führt `MpCmdRun.exe` aus; diese Bibliothek hält die
//! Entscheidungslogik testbar und frei von Pfaden oder Prozessaufrufen.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Clean,
    Threat,
    Unknown,
}

/// `MpCmdRun` exit codes. A scan that failed with HRESULT is `Unknown`, not
/// clean, so the caller can keep the file in the ingress folder.
pub fn parse_defender_exit(code: Option<i32>, output: &str) -> Verdict {
    let text = output.to_ascii_lowercase();
    if text.contains("failed with hr") {
        return Verdict::Unknown;
    }
    match code {
        Some(0) => Verdict::Clean,
        Some(2) => Verdict::Threat,
        _ => Verdict::Unknown,
    }
}

/// Standard install locations, checked without PATH lookup.
pub fn defender_path() -> Option<PathBuf> {
    [
        r"C:\Program Files\Windows Defender\MpCmdRun.exe",
        r"C:\Program Files (x86)\Windows Defender\MpCmdRun.exe",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|path| path.is_file())
}

/// Extensions that always deserve a warning even when the scan is clean.
pub fn is_dangerous_extension(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "exe" | "scr" | "lnk" | "js" | "vbs" | "msi" | "iso" | "docm" | "xlsm"
    )
}

/// Double extensions such as `invoice.pdf.exe`.
pub fn has_double_extension(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() < 3 {
        return false;
    }
    let last = parts.last().copied().unwrap_or("");
    matches!(
        last,
        "exe" | "scr" | "lnk" | "js" | "vbs" | "msi" | "iso" | "docm" | "xlsm"
    ) && parts[parts.len() - 2] != last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defender_codes_are_mapped() {
        assert_eq!(parse_defender_exit(Some(0), ""), Verdict::Clean);
        assert_eq!(parse_defender_exit(Some(2), ""), Verdict::Threat);
        assert_eq!(parse_defender_exit(Some(3), ""), Verdict::Unknown);
        assert_eq!(
            parse_defender_exit(Some(0), "Scan failed with HRESULT 0x80004005"),
            Verdict::Unknown
        );
    }

    #[test]
    fn dangerous_extensions_are_flagged() {
        assert!(is_dangerous_extension(Path::new("setup.exe")));
        assert!(is_dangerous_extension(Path::new("sheet.XLSM")));
        assert!(!is_dangerous_extension(Path::new("photo.jpg")));
        assert!(!is_dangerous_extension(Path::new("README")));
    }

    #[test]
    fn double_extension_is_detected() {
        assert!(has_double_extension(Path::new("invoice.pdf.exe")));
        assert!(has_double_extension(Path::new("photo.jpg.scr")));
        assert!(!has_double_extension(Path::new("archive.tar.gz")));
        assert!(!has_double_extension(Path::new("setup.exe")));
    }
}
