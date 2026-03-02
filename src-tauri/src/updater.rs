use serde::Serialize;
use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;

/// Result of checking for an available update.
/// Returned to the frontend so it can decide whether to show a notification.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub version: String,
    pub body: String,
}

/// Check the GitHub Releases endpoint for a newer version.
///
/// Returns { available: true, version, body } if an update exists,
/// or { available: false, version: "", body: "" } if already on latest.
///
/// The frontend calls this on launch (after a short delay) and periodically.
/// Downloading/installing is handled entirely by the JS plugin API (check() + downloadAndInstall()).
#[tauri::command]
pub async fn check_for_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let updater = app.updater().map_err(|e| format!("Updater not available: {}", e))?;
    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateInfo {
            available: true,
            version: update.version.clone(),
            body: update.body.clone().unwrap_or_default(),
        }),
        Ok(None) => Ok(UpdateInfo {
            available: false,
            version: String::new(),
            body: String::new(),
        }),
        Err(e) => {
            log::warn!("Update check failed: {}", e);
            Err(format!("Update check failed: {}", e))
        }
    }
}
