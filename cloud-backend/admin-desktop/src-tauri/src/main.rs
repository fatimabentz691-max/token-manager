#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const REPOSITORY: &str = "fatimabentz691-max/HUSSEL";
const GITHUB_DOWNLOAD_ROOT: &str = "https://github.com/fatimabentz691-max/HUSSEL/releases/download";

#[derive(Debug, Serialize)]
struct InstallerInfo {
    file_name: String,
    publish_file_name: String,
    path: String,
    version: String,
    size_bytes: u64,
    sha256: String,
    signature_path: String,
    signature: String,
}

#[derive(Clone, Debug, Deserialize)]
struct PublishRequest {
    path: String,
    version: String,
    title: String,
    notes: String,
    #[serde(default)]
    highlights: Vec<String>,
    #[serde(default)]
    fixes: Vec<String>,
    admin_key: String,
    backend_url: String,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct PublishResult {
    version: String,
    download_url: String,
    file_name: String,
    size_bytes: u64,
    sha256: String,
    updater_url: String,
    signature: String,
}

#[derive(Debug, Serialize)]
struct PublishEnvironment {
    ready: bool,
    github_account: String,
    repository: String,
    message: String,
}

fn version_from_name(name: &str) -> String {
    let name = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name);
    let mut candidates = Vec::new();
    for (index, ch) in name.char_indices() {
        if ch.is_ascii_digit() {
            let suffix = &name[index..];
            let candidate: String = suffix
                .chars()
                .take_while(|value| {
                    value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '+')
                })
                .collect();
            let core = candidate.split(['-', '+']).next().unwrap_or("");
            if core.split('.').count() == 3
                && core.split('.').all(|part| {
                    !part.is_empty() && part.chars().all(|value| value.is_ascii_digit())
                })
            {
                candidates.push(candidate.trim_end_matches(['.', '-', '+']).to_string());
            }
        }
    }
    candidates.into_iter().next().unwrap_or_default()
}

fn normalized_version(value: &str) -> Result<String, String> {
    let version = value.trim().trim_start_matches(['v', 'V']);
    let core = version.split(['-', '+']).next().unwrap_or("");
    if core.split('.').count() != 3
        || !core
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|value| value.is_ascii_digit()))
    {
        return Err("版本号格式应为 0.10.4".into());
    }
    if !version
        .chars()
        .all(|value| value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '+'))
    {
        return Err("版本号包含不支持的字符".into());
    }
    Ok(version.to_string())
}

fn hash_file(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|error| format!("无法读取安装包：{error}"))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| format!("读取安装包失败：{error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:X}", hasher.finalize()))
}

fn find_updater_signature(path: &Path) -> Result<(PathBuf, String), String> {
    let signature_path = PathBuf::from(format!("{}.sig", path.to_string_lossy()));
    let signature = fs::read_to_string(&signature_path)
        .map_err(|_| "缺少更新签名（*.exe.sig），已禁止发布未签名更新。".to_string())?
        .trim()
        .to_string();
    if signature.is_empty() {
        return Err("更新签名为空，已禁止发布。".into());
    }
    Ok((signature_path, signature))
}

fn inspect_path(path: &Path) -> Result<InstallerInfo, String> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("exe"))
        != Some(true)
    {
        return Err("仅支持 Windows .exe 安装包".into());
    }
    let metadata =
        fs::metadata(path).map_err(|error| format!("安装包不存在或无法读取：{error}"))?;
    if metadata.len() == 0 || metadata.len() > 1024 * 1024 * 1024 {
        return Err("安装包大小必须在 1 字节至 1 GB 之间".into());
    }
    let mut signature = [0_u8; 2];
    File::open(path)
        .and_then(|mut file| file.read_exact(&mut signature))
        .map_err(|error| format!("无法检查安装包格式：{error}"))?;
    if signature != *b"MZ" {
        return Err("该文件不是有效的 Windows PE 安装包".into());
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("TokenManager-setup.exe")
        .to_string();
    if file_name.to_ascii_lowercase().contains("admin") {
        return Err("这是 Token Manager Admin 后台安装包，不能作为 Token Manager 主程序上架。请拖入主程序安装包。".into());
    }
    let version = version_from_name(&file_name);
    let publish_version = if version.is_empty() {
        "0.0.0".to_string()
    } else {
        version.clone()
    };
    let (signature_path, updater_signature) = find_updater_signature(path)?;
    Ok(InstallerInfo {
        file_name,
        publish_file_name: format!("TokenManager_{publish_version}_x64-setup.exe"),
        path: path.to_string_lossy().to_string(),
        version,
        size_bytes: metadata.len(),
        sha256: hash_file(path)?,
        signature_path: signature_path.to_string_lossy().to_string(),
        signature: updater_signature,
    })
}

#[cfg(windows)]
fn hidden_output(command: &mut Command) -> std::io::Result<Output> {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000).output()
}

#[cfg(not(windows))]
fn hidden_output(command: &mut Command) -> std::io::Result<Output> {
    command.output()
}

fn github_cli() -> PathBuf {
    let installed = PathBuf::from(r"C:\Program Files\GitHub CLI\gh.exe");
    if installed.exists() {
        installed
    } else {
        PathBuf::from("gh")
    }
}

fn run_gh(arguments: &[&str]) -> Result<Output, String> {
    let mut command = Command::new(github_cli());
    command
        .args(arguments)
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .env("NO_COLOR", "1");
    hidden_output(&mut command)
        .map_err(|error| format!("无法运行 GitHub CLI：{error}\n请安装 GitHub CLI，并在 PowerShell 执行 gh auth login 完成登录。"))
}

fn require_success(output: Output, action: &str) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    Err(format!(
        "{action}失败：{}",
        if detail.is_empty() {
            "GitHub CLI 未返回详细原因"
        } else {
            &detail
        }
    ))
}

fn output_text(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stdout.is_empty() {
        stdout
    } else {
        stderr
    }
}

fn inspect_publish_environment() -> Result<PublishEnvironment, String> {
    require_success(
        run_gh(&["auth", "status", "--hostname", "github.com"])?,
        "GitHub 登录检查",
    )?;
    require_success(
        run_gh(&["repo", "view", REPOSITORY, "--json", "nameWithOwner"])?,
        "GitHub 仓库权限检查",
    )?;
    let account_output = run_gh(&["api", "user", "--jq", ".login"])?;
    require_success(
        Output {
            status: account_output.status,
            stdout: account_output.stdout.clone(),
            stderr: account_output.stderr.clone(),
        },
        "GitHub 账户检查",
    )?;
    Ok(PublishEnvironment {
        ready: true,
        github_account: output_text(&account_output),
        repository: REPOSITORY.to_string(),
        message: "GitHub 登录与仓库写入链路可用".into(),
    })
}

fn publish_to_github(
    request: &PublishRequest,
    version: &str,
) -> Result<(String, String, String, String), String> {
    inspect_publish_environment()?;
    let tag = format!("v{version}");
    let publish_name = format!("TokenManager_{version}_x64-setup.exe");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let temp_dir = std::env::temp_dir().join(format!("token-manager-admin-{stamp}"));
    fs::create_dir_all(&temp_dir).map_err(|error| format!("无法创建发布缓存：{error}"))?;
    let asset_path = temp_dir.join(&publish_name);
    let updater_signature_path = temp_dir.join(format!("{publish_name}.sig"));
    let notes_path = temp_dir.join("release-notes.md");
    let info = inspect_path(Path::new(&request.path))?;
    fs::copy(&info.signature_path, &updater_signature_path)
        .map_err(|error| format!("无法准备更新签名：{error}"))?;
    fs::copy(&request.path, &asset_path).map_err(|error| format!("无法准备发布文件：{error}"))?;
    fs::write(
        &notes_path,
        if request.notes.trim().is_empty() {
            format!("Token Manager v{version}")
        } else {
            request.notes.clone()
        },
    )
    .map_err(|error| format!("无法准备版本说明：{error}"))?;
    let tag_ref = tag.as_str();
    let title = if request.title.trim().is_empty() {
        format!("Token Manager v{version}")
    } else {
        request.title.trim().to_string()
    };
    let notes = notes_path.to_string_lossy().to_string();
    let exists = run_gh(&["release", "view", tag_ref, "--repo", REPOSITORY])?
        .status
        .success();
    let release_result = if exists {
        run_gh(&[
            "release",
            "edit",
            tag_ref,
            "--repo",
            REPOSITORY,
            "--title",
            &title,
            "--notes-file",
            &notes,
        ])?
    } else {
        run_gh(&[
            "release",
            "create",
            tag_ref,
            "--repo",
            REPOSITORY,
            "--target",
            "main",
            "--title",
            &title,
            "--notes-file",
            &notes,
        ])?
    };
    require_success(release_result, "创建或更新 GitHub Release")?;
    let asset = asset_path.to_string_lossy().to_string();
    let updater_signature_asset = updater_signature_path.to_string_lossy().to_string();
    let upload_result = run_gh(&[
        "release",
        "upload",
        tag_ref,
        &asset,
        &updater_signature_asset,
        "--repo",
        REPOSITORY,
        "--clobber",
    ])?;
    let upload_status = require_success(upload_result, "上传安装包到 GitHub Release");
    let _ = fs::remove_dir_all(&temp_dir);
    upload_status?;
    let download_url = format!("{GITHUB_DOWNLOAD_ROOT}/{tag}/{publish_name}");
    let updater_url = download_url.clone();
    Ok((publish_name, download_url, updater_url, info.signature))
}

#[tauri::command]
fn inspect_installer(path: String) -> Result<InstallerInfo, String> {
    inspect_path(Path::new(&path))
}

#[tauri::command]
async fn check_publish_environment() -> Result<PublishEnvironment, String> {
    tauri::async_runtime::spawn_blocking(inspect_publish_environment)
        .await
        .map_err(|error| format!("发布环境检查任务失败：{error}"))?
}

#[tauri::command]
async fn publish_installer(request: PublishRequest) -> Result<PublishResult, String> {
    let version = normalized_version(&request.version)?;
    if request.admin_key.trim().is_empty() {
        return Err("请先输入后台管理密钥".into());
    }
    let backend_url = request.backend_url.trim_end_matches('/').to_string();
    if !backend_url.starts_with("https://")
        && !backend_url.starts_with("http://127.0.0.1")
        && !backend_url.starts_with("http://localhost")
    {
        return Err("后台地址必须使用 HTTPS".into());
    }
    let path = request.path.clone();
    let info = tauri::async_runtime::spawn_blocking(move || inspect_path(Path::new(&path)))
        .await
        .map_err(|error| format!("文件检查任务失败：{error}"))??;
    let github_request = PublishRequest {
        version: version.clone(),
        ..request
    };
    let blocking_request = github_request.clone();
    let blocking_version = version.clone();
    let (file_name, download_url, updater_url, signature) =
        tauri::async_runtime::spawn_blocking(move || {
            publish_to_github(&blocking_request, &blocking_version)
        })
        .await
        .map_err(|error| format!("发布任务失败：{error}"))??;
    let version = normalized_version(&github_request.version)?;
    let response = reqwest::Client::new()
        .post(format!("{backend_url}/v1/admin/release"))
        .header("x-admin-key", github_request.admin_key.trim())
        .json(&serde_json::json!({
            "version": version,
            "title": github_request.title,
            "download_url": download_url,
            "updater_url": updater_url,
            "file_name": file_name,
            "size_bytes": info.size_bytes,
            "sha256": info.sha256,
            "signature": signature,
            "channel": "stable",
            "platform": "windows-x86_64",
            "notes": github_request.notes,
            "highlights": github_request.highlights,
            "fixes": github_request.fixes,
            "enabled": github_request.enabled
        }))
        .send()
        .await
        .map_err(|error| format!("安装包已上传 GitHub，但官网更新失败：{error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let message = response.text().await.unwrap_or_default();
        return Err(format!(
            "安装包已上传 GitHub，但官网更新失败（{status}）：{message}"
        ));
    }
    Ok(PublishResult {
        version,
        download_url,
        file_name,
        size_bytes: info.size_bytes,
        sha256: info.sha256,
        updater_url,
        signature,
    })
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            inspect_installer,
            check_publish_environment,
            publish_installer
        ])
        .run(tauri::generate_context!())
        .expect("Token Manager Admin 启动失败");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_release_version() {
        assert_eq!(
            version_from_name("Token Manager_0.10.4_x64-setup.exe"),
            "0.10.4"
        );
        assert_eq!(
            version_from_name("TokenManager-v1.2.3-beta.exe"),
            "1.2.3-beta"
        );
    }

    #[test]
    fn rejects_invalid_release_version() {
        assert!(normalized_version("0.10").is_err());
        assert!(normalized_version("0.10.4/evil").is_err());
    }

    #[test]
    fn rejects_admin_installer() {
        let path = std::env::temp_dir().join("Token Manager Admin_0.10.5_x64-setup.exe");
        fs::write(&path, b"MZtest").expect("create fixture");
        let result = inspect_path(&path);
        let _ = fs::remove_file(path);
        assert!(result.unwrap_err().contains("后台安装包"));
    }
}
