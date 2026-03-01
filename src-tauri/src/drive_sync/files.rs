use google_drive3::api::File as DriveFile;
use google_drive3::DriveHub;
use log::info;
use std::path::Path;

use super::auth::DriveAuthenticator;

/// Name of the root sync folder on Google Drive.
const SYNC_FOLDER_NAME: &str = "KanColle Browser Sync";
/// MIME type for Google Drive folders.
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
/// Scope to use for all Drive API calls (must match the OAuth token scope).
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";

/// The connector type used by our Hub.
type Connector = hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>;
pub type Hub = DriveHub<Connector>;

/// Build a DriveHub from an authenticator.
pub fn build_hub(auth: DriveAuthenticator) -> Hub {
    let client = hyper_util::client::legacy::Client::builder(
        hyper_util::rt::TokioExecutor::new(),
    )
    .build(
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .unwrap()
            .https_or_http()
            .enable_http1()
            .build(),
    );
    DriveHub::new(client, auth)
}

/// Ensure the root sync folder exists on Drive. Returns the folder ID.
pub async fn ensure_sync_folder(hub: &Hub) -> Result<String, String> {
    let query = format!(
        "name = '{}' and mimeType = '{}' and trashed = false",
        SYNC_FOLDER_NAME, FOLDER_MIME
    );
    let (_, file_list) = hub
        .files()
        .list()
        .q(&query)
        .spaces("drive")
        .param("fields", "files(id, name)")
        .add_scope(DRIVE_SCOPE)
        .doit()
        .await
        .map_err(|e| format!("Failed to search for sync folder: {}", e))?;

    if let Some(files) = file_list.files {
        if let Some(folder) = files.first() {
            if let Some(id) = &folder.id {
                info!("Found existing sync folder: {}", id);
                return Ok(id.clone());
            }
        }
    }

    // Create new folder
    let folder = DriveFile {
        name: Some(SYNC_FOLDER_NAME.to_string()),
        mime_type: Some(FOLDER_MIME.to_string()),
        ..Default::default()
    };
    let (_, created) = hub
        .files()
        .create(folder)
        .param("fields", "id")
        .add_scope(DRIVE_SCOPE)
        .upload(
            std::io::empty(),
            "application/octet-stream".parse::<mime::Mime>().unwrap(),
        )
        .await
        .map_err(|e| format!("Failed to create sync folder: {}", e))?;

    let id = created.id.ok_or("Created folder has no ID")?;
    info!("Created sync folder: {}", id);
    Ok(id)
}

/// Ensure a subfolder exists inside the sync folder. Returns the subfolder ID.
pub async fn ensure_subfolder(hub: &Hub, parent_id: &str, name: &str) -> Result<String, String> {
    let query = format!(
        "name = '{}' and mimeType = '{}' and '{}' in parents and trashed = false",
        name, FOLDER_MIME, parent_id
    );
    let (_, file_list) = hub
        .files()
        .list()
        .q(&query)
        .spaces("drive")
        .param("fields", "files(id, name)")
        .add_scope(DRIVE_SCOPE)
        .doit()
        .await
        .map_err(|e| format!("Failed to search for subfolder '{}': {}", name, e))?;

    if let Some(files) = file_list.files {
        if let Some(folder) = files.first() {
            if let Some(id) = &folder.id {
                return Ok(id.clone());
            }
        }
    }

    let folder = DriveFile {
        name: Some(name.to_string()),
        mime_type: Some(FOLDER_MIME.to_string()),
        parents: Some(vec![parent_id.to_string()]),
        ..Default::default()
    };
    let (_, created) = hub
        .files()
        .create(folder)
        .param("fields", "id")
        .add_scope(DRIVE_SCOPE)
        .upload(
            std::io::empty(),
            "application/octet-stream".parse::<mime::Mime>().unwrap(),
        )
        .await
        .map_err(|e| format!("Failed to create subfolder '{}': {}", name, e))?;

    let id = created.id.ok_or("Created subfolder has no ID")?;
    info!("Created subfolder '{}': {}", name, id);
    Ok(id)
}

/// Upload a file to Google Drive. Returns the file ID and modified time.
pub async fn upload_file(
    hub: &Hub,
    parent_id: &str,
    file_name: &str,
    local_path: &Path,
    existing_file_id: Option<&str>,
) -> Result<(String, chrono::DateTime<chrono::Utc>), String> {
    let content = std::fs::read(local_path)
        .map_err(|e| format!("Failed to read '{}': {}", local_path.display(), e))?;
    let mime: mime::Mime = "application/octet-stream".parse().unwrap();

    if let Some(fid) = existing_file_id {
        let file_meta = DriveFile::default();
        let (_, updated) = hub
            .files()
            .update(file_meta, fid)
            .param("fields", "id, modifiedTime")
            .add_scope(DRIVE_SCOPE)
            .upload(std::io::Cursor::new(content), mime)
            .await
            .map_err(|e| format!("Failed to update '{}': {}", file_name, e))?;

        let modified = updated.modified_time.unwrap_or_else(chrono::Utc::now);
        Ok((fid.to_string(), modified))
    } else {
        let file_meta = DriveFile {
            name: Some(file_name.to_string()),
            parents: Some(vec![parent_id.to_string()]),
            ..Default::default()
        };
        let (_, created) = hub
            .files()
            .create(file_meta)
            .param("fields", "id, modifiedTime")
            .add_scope(DRIVE_SCOPE)
            .upload(std::io::Cursor::new(content), mime)
            .await
            .map_err(|e| format!("Failed to create '{}': {}", file_name, e))?;

        let id = created.id.ok_or("Created file has no ID")?;
        let modified = created.modified_time.unwrap_or_else(chrono::Utc::now);
        info!("Uploaded '{}': {}", file_name, id);
        Ok((id, modified))
    }
}

/// Download a file from Google Drive to a local path.
/// Uses alt=media to get raw file content in the response body.
pub async fn download_file(hub: &Hub, file_id: &str, local_path: &Path) -> Result<(), String> {
    let (response, _file) = hub
        .files()
        .get(file_id)
        .param("alt", "media")
        .add_scope(DRIVE_SCOPE)
        .doit()
        .await
        .map_err(|e| format!("Failed to download '{}': {}", file_id, e))?;

    // With alt=media, the response body contains the raw file content
    use http_body_util::BodyExt;
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Failed to read body: {}", e))?
        .to_bytes();

    if let Some(parent) = local_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(local_path, &body_bytes)
        .map_err(|e| format!("Failed to write '{}': {}", local_path.display(), e))?;

    Ok(())
}

/// Represents a remote file on Drive.
pub struct RemoteFile {
    pub id: String,
    pub name: String,
    pub modified_time: chrono::DateTime<chrono::Utc>,
    pub md5: Option<String>,
}

/// List all files in a folder on Drive.
pub async fn list_files(hub: &Hub, folder_id: &str) -> Result<Vec<RemoteFile>, String> {
    let mut all_files = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let query = format!(
            "'{}' in parents and mimeType != '{}' and trashed = false",
            folder_id, FOLDER_MIME
        );
        let mut req = hub
            .files()
            .list()
            .q(&query)
            .spaces("drive")
            .param(
                "fields",
                "nextPageToken, files(id, name, modifiedTime, md5Checksum)",
            )
            .page_size(1000)
            .add_scope(DRIVE_SCOPE);

        if let Some(ref token) = page_token {
            req = req.page_token(token);
        }

        let (_, file_list) = req
            .doit()
            .await
            .map_err(|e| format!("Failed to list files in '{}': {}", folder_id, e))?;

        if let Some(files) = file_list.files {
            for f in files {
                let id = match f.id {
                    Some(id) => id,
                    None => continue,
                };
                all_files.push(RemoteFile {
                    id,
                    name: f.name.unwrap_or_default(),
                    modified_time: f.modified_time.unwrap_or_else(chrono::Utc::now),
                    md5: f.md5_checksum,
                });
            }
        }

        page_token = file_list.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    Ok(all_files)
}

/// Delete a file from Google Drive.
pub async fn delete_file(hub: &Hub, file_id: &str) -> Result<(), String> {
    hub.files()
        .delete(file_id)
        .add_scope(DRIVE_SCOPE)
        .doit()
        .await
        .map_err(|e| format!("Failed to delete '{}': {}", file_id, e))?;
    Ok(())
}
