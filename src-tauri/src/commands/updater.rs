use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::events::UPDATE_PROGRESS;

// GitHub config
const GITHUB_OWNER: &str = "FiveDayZ";
const GITHUB_REPO: &str = "CoworkPal";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub changelog: String,
    pub download_url: String,
    pub asset_id: Option<u64>,
    pub asset_size: u64,
    pub asset_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percent: f64,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    id: u64,
    name: String,
    size: u64,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GithubAsset>,
}

#[tauri::command]
pub async fn check_update(pat: Option<String>) -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let client = reqwest::Client::builder()
        .user_agent("CoworkPal-Updater")
        .timeout(std::time::Duration::from_secs(20))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let api_url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );

    let mut request = client.get(&api_url);

    // Private repo authentication support
    if let Some(ref token) = pat {
        if !token.is_empty() {
            request = request.header("Authorization", format!("token {}", token));
        }
    } else if let Ok(env_token) = std::env::var("GITHUB_TOKEN") {
        if !env_token.is_empty() {
            request = request.header("Authorization", format!("token {}", env_token));
        }
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to send check update request: {}", e))?;

    if response.status() == 403 {
        return Err("GitHub API rate limit exceeded or access forbidden. Please configure a GitHub Token.".to_string());
    }

    if response.status() == 404 {
        return Err("GitHub API returned 404 Not Found. If the repository is private, please provide a valid Access Token. If it is public, please check if the GitHub Actions build has completed and a Release is published.".to_string());
    }

    if !response.status().is_success() {
        return Err(format!(
            "GitHub API returned error status: {}",
            response.status()
        ));
    }

    let release: GithubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release JSON: {}", e))?;

    // Version comparison
    let current_semver = semver::Version::parse(&current_version)
        .unwrap_or_else(|_| semver::Version::new(0, 1, 0));
    
    let latest_tag_clean = release.tag_name.trim_start_matches('v');
    let latest_semver = semver::Version::parse(latest_tag_clean)
        .map_err(|e| format!("Invalid online release version format '{}': {}", latest_tag_clean, e))?;

    let has_update = latest_semver > current_semver;

    // Look for Windows installer (.exe setup, .msi, or .zip)
    let matched_asset = release.assets.iter().find(|a| {
        a.name.ends_with(".exe") || a.name.ends_with(".msi") || a.name.ends_with(".zip")
    });

    match matched_asset {
        Some(asset) => Ok(UpdateCheckResult {
            has_update,
            current_version,
            latest_version: release.tag_name.clone(),
            changelog: release.body.unwrap_or_default(),
            download_url: asset.browser_download_url.clone(),
            asset_id: Some(asset.id),
            asset_size: asset.size,
            asset_name: asset.name.clone(),
        }),
        None => Err("No valid release assets (.exe, .msi, or .zip) found in the latest release.".to_string()),
    }
}

#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    asset_id: u64,
    asset_name: String,
    pat: Option<String>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("CoworkPal-Updater")
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    // Determine download URL
    // For private repo, we must request via the assets API to get the octet-stream
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/assets/{}",
        GITHUB_OWNER, GITHUB_REPO, asset_id
    );

    // Temp directory resolution
    let temp_dir = std::env::temp_dir().join("CoworkPal-Updates");
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create update directory: {}", e))?;
    
    let part_path = temp_dir.join(format!("{}.part", asset_name));
    let final_path = temp_dir.join(&asset_name);

    // Get current size for range request (Breakpoint Resume)
    let start_pos = fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0);

    let mut request = client.get(&url)
        .header("Accept", "application/octet-stream");

    if let Some(ref token) = pat {
        if !token.is_empty() {
            request = request.header("Authorization", format!("token {}", token));
        }
    } else if let Ok(env_token) = std::env::var("GITHUB_TOKEN") {
        if !env_token.is_empty() {
            request = request.header("Authorization", format!("token {}", env_token));
        }
    }

    if start_pos > 0 {
        request = request.header("Range", format!("bytes={}-", start_pos));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to send download request: {}", e))?;

    let status = response.status();
    let is_partial = status == reqwest::StatusCode::PARTIAL_CONTENT;

    if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(format!("Download request failed with status: {}", status));
    }

    // Open file in append or truncate mode
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(is_partial)
        .open(&part_path)
        .map_err(|e| format!("Failed to open local destination file: {}", e))?;

    if !is_partial {
        // If server didn't respond with 206, truncate the file and download from start.
        // A failed truncate must abort: otherwise leftover bytes from a prior
        // partial download would be prepended to the fresh stream, corrupting it.
        file.set_len(0)
            .map_err(|e| format!("Failed to truncate partial download file: {}", e))?;
    }

    let mut downloaded = if is_partial { start_pos } else { 0 };
    let content_len = response.content_length().unwrap_or(0);
    let total_bytes = if is_partial {
        content_len + start_pos
    } else {
        content_len
    };

    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Error during stream chunk read: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk to disk: {}", e))?;
        
        downloaded += chunk.len() as u64;

        let percent = if total_bytes > 0 {
            (downloaded as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        // Emit download progress to frontend
        app.emit(
            UPDATE_PROGRESS,
            DownloadProgress {
                bytes_downloaded: downloaded,
                total_bytes,
                percent,
            },
        )
        .ok();
    }

    // Explicitly flush and close the file
    file.sync_all().map_err(|e| format!("Failed to sync file to disk: {}", e))?;
    drop(file);

    // Rename file to final name
    if final_path.exists() {
        fs::remove_file(&final_path).ok();
    }
    fs::rename(&part_path, &final_path)
        .map_err(|e| format!("Failed to rename completed download to final path: {}", e))?;

    Ok(final_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn install_update(app: AppHandle, package_path: String) -> Result<(), String> {
    let path = PathBuf::from(&package_path);
    if !path.exists() {
        return Err(format!("Update package not found at path: {}", package_path));
    }

    // Verify file integrity when a SHA256 sidecar is present next to the package.
    // The sidecar is optional today (releases may not ship one), but when it
    // exists we enforce it; when absent we log and proceed.
    let sha256_path = path.with_extension(format!("{}.sha256", path.extension().unwrap_or_default().to_string_lossy()));
    if sha256_path.exists() {
        let mut sha256_file = File::open(&sha256_path).map_err(|e| e.to_string())?;
        let mut expected_hash = String::new();
        sha256_file.read_to_string(&mut expected_hash).map_err(|e| e.to_string())?;
        let expected_hash = expected_hash.trim().to_lowercase();

        let mut file = File::open(&path).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        loop {
            let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        let calculated_hash = hex::encode(hasher.finalize());
        if calculated_hash != expected_hash {
            return Err("File verification failed: SHA256 checksum mismatch.".to_string());
        }
        tracing::info!("update package passed SHA256 verification: {}", package_path);
    } else {
        tracing::warn!(
            "no SHA256 sidecar found for update package {}; skipping integrity verification: {}",
            package_path,
            sha256_path.display()
        );
    }

    // Windows upgrade action
    #[cfg(windows)]
    {
        let is_installer = package_path.ends_with(".exe") || package_path.ends_with(".msi");

        if is_installer {
            // It's a setup installer. Launch it and exit the application
            // Use shell execute to handle UAC elevation automatically if installer requests it
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            // NSIS installers support /S for silent install, MSI supports /qn, but here we run interactive so user sees install wizard
            let mut command = std::process::Command::new("cmd.exe");
            command.args(["/c", "start", "", &package_path]);
            command.creation_flags(CREATE_NO_WINDOW);
            
            command.spawn()
                .map_err(|e| format!("Failed to spawn installer process: {}", e))?;
            
            // Terminate current process to let the installer replace the file.
            // Use app.exit(0) so Tauri runs its graceful shutdown (window
            // cleanup, drop handlers) instead of a hard process::exit.
            app.exit(0);
        } else if package_path.ends_with(".zip") {
            // Portable Zip-based hot swap
            // Extract the Zip and replace current exe. Note: Zip extraction requires external tools or library.
            // Since we package as NSIS setup.exe by default on Windows, let's focus on NSIS / MSI execution.
            // If they pass a raw executable renamed as .exe, we can do hot-swap:
            return Err("Zip updates not supported in standard installer mode. Please package as .exe or .msi setup.".to_string());
        }
    }

    #[cfg(not(windows))]
    {
        return Err("Update installation is only supported on Windows platforms.".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_update_fails_gracefully_with_bad_token() {
        tauri::async_runtime::block_on(async {
            let res = check_update(Some("bad_token_example".to_string())).await;
            assert!(res.is_err());
            let err_msg = res.err().unwrap();
            assert!(
                err_msg.contains("GitHub API returned error status")
                    || err_msg.contains("forbidden")
                    || err_msg.contains("rate limit")
                    || err_msg.contains("forbidden")
            );
        });
    }

    #[test]
    fn test_check_update_anonymous() {
        tauri::async_runtime::block_on(async {
            let res = check_update(None).await;
            match res {
                Ok(val) => {
                    println!("Anonymous check succeeded, latest version: {}", val.latest_version);
                    assert!(!val.latest_version.is_empty());
                }
                Err(e) => {
                    println!("Anonymous check returned error: {}", e);
                    let normalized = e.to_ascii_lowercase();
                    assert!(
                        normalized.contains("rate limit")
                            || normalized.contains("error status")
                            || normalized.contains("failed to send")
                            || normalized.contains("no valid release assets")
                            || normalized.contains("not found")
                    );
                }
            }
        });
    }
}
