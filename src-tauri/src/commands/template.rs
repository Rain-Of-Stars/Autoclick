use autoclick_diagnostics::error_code::ErrorCode;
use autoclick_domain::template::TemplateRef;
use autoclick_storage::template_fs::{
    import_template_bytes, import_template_file, remove_managed_template_file,
};
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    app_state::AppState,
    commands::error::{CommandResult, command_error},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTemplateRequest {
    pub file_path: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameTemplateRequest {
    pub template_id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPastedTemplateRequest {
    pub bytes: Vec<u8>,
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePreviewRequest {
    pub template_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePreviewPayload {
    pub mime_type: String,
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub fn list_templates(state: State<'_, AppState>) -> CommandResult<Vec<TemplateRef>> {
    state
        .list_templates()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))
}

#[tauri::command]
pub fn get_template_preview(
    state: State<'_, AppState>,
    request: TemplatePreviewRequest,
) -> CommandResult<TemplatePreviewPayload> {
    let template = state
        .list_templates()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?
        .into_iter()
        .find(|item| item.id.to_string() == request.template_id)
        .ok_or_else(|| command_error(ErrorCode::StorageUnavailable, "未找到指定模板"))?;
    let template_path = template
        .stored_path
        .as_deref()
        .or(template.source_path.as_deref())
        .ok_or_else(|| command_error(ErrorCode::StorageUnavailable, "模板缺少可读取路径"))?;
    let bytes = std::fs::read(template_path).map_err(|err| {
        command_error(
            ErrorCode::StorageUnavailable,
            format!("读取模板预览失败: {err}"),
        )
    })?;

    if bytes.is_empty() {
        return Err(command_error(
            ErrorCode::StorageUnavailable,
            "模板文件为空，无法生成预览",
        ));
    }

    Ok(TemplatePreviewPayload {
        mime_type: detect_mime_type(template_path, &bytes),
        bytes,
        width: template.width,
        height: template.height,
    })
}

#[tauri::command]
pub fn import_template(
    state: State<'_, AppState>,
    request: ImportTemplateRequest,
) -> CommandResult<TemplateRef> {
    let app_paths = state
        .app_paths()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let repository = state
        .template_repository()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;

    let file_path = request.file_path.trim();
    if file_path.is_empty() {
        return Err(command_error(
            ErrorCode::StorageUnavailable,
            "模板文件路径不能为空",
        ));
    }
    let tags = request
        .tags
        .iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let template = import_template_file(&app_paths, file_path, &tags)
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err.to_string()))?;
    repository
        .upsert(&template)
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err.to_string()))?;
    state.runtime.invalidate_template_cache(&template.hash);
    let _ = state.sync_templates_into_config();
    Ok(template)
}

#[tauri::command]
pub fn import_pasted_template(
    state: State<'_, AppState>,
    request: ImportPastedTemplateRequest,
) -> CommandResult<TemplateRef> {
    let app_paths = state
        .app_paths()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let repository = state
        .template_repository()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let name = request.name.trim();
    if name.is_empty() {
        return Err(command_error(
            ErrorCode::StorageUnavailable,
            "模板名称不能为空",
        ));
    }
    if request.bytes.is_empty() {
        return Err(command_error(
            ErrorCode::StorageUnavailable,
            "粘贴内容为空，无法导入模板",
        ));
    }
    if request.bytes.len() > 25 * 1024 * 1024 {
        return Err(command_error(
            ErrorCode::StorageUnavailable,
            "粘贴图片过大，请压缩后再导入",
        ));
    }
    let tags = request
        .tags
        .iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let template = import_template_bytes(
        &app_paths,
        &request.bytes,
        name.to_string(),
        Some("clipboard://image".to_string()),
        &tags,
    )
    .map_err(|err| command_error(ErrorCode::StorageUnavailable, err.to_string()))?;
    repository
        .upsert(&template)
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err.to_string()))?;
    state.runtime.invalidate_template_cache(&template.hash);
    let _ = state.sync_templates_into_config();
    Ok(template)
}

#[tauri::command]
pub fn remove_template(
    state: State<'_, AppState>,
    template_id: String,
) -> CommandResult<Vec<TemplateRef>> {
    let app_paths = state
        .app_paths()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let repository = state
        .template_repository()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let templates = repository
        .list()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err.to_string()))?;
    let template = templates
        .iter()
        .find(|item| item.id.to_string() == template_id)
        .cloned()
        .ok_or_else(|| command_error(ErrorCode::StorageUnavailable, "未找到指定模板"))?;

    if let Some(stored_path) = template.stored_path.as_deref() {
        remove_managed_template_file(&app_paths, stored_path)
            .map_err(|err| command_error(ErrorCode::StorageUnavailable, err.to_string()))?;
    }
    state.runtime.invalidate_template_cache(&template.hash);
    repository
        .delete(template.id)
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err.to_string()))?;
    state
        .sync_templates_into_config()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))
}

#[tauri::command]
pub fn rename_template(
    state: State<'_, AppState>,
    request: RenameTemplateRequest,
) -> CommandResult<TemplateRef> {
    let repository = state
        .template_repository()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let mut templates = repository
        .list()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err.to_string()))?;
    let next_name = request.name.trim();
    if next_name.is_empty() {
        return Err(command_error(
            ErrorCode::StorageUnavailable,
            "模板名称不能为空",
        ));
    }
    let template = templates
        .iter_mut()
        .find(|item| item.id.to_string() == request.template_id)
        .ok_or_else(|| command_error(ErrorCode::StorageUnavailable, "未找到指定模板"))?;
    template.name = next_name.to_string();
    repository
        .upsert(template)
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err.to_string()))?;
    state.runtime.invalidate_template_cache(&template.hash);
    let _ = state.sync_templates_into_config();
    Ok(template.clone())
}

fn detect_mime_type(path: &str, bytes: &[u8]) -> String {
    if let Ok(format) = image::guess_format(bytes) {
        return image_format_to_mime(format).to_string();
    }

    match std::path::Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("bmp") => "image/bmp",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("tif") | Some("tiff") => "image/tiff",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn image_format_to_mime(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Gif => "image/gif",
        ImageFormat::WebP => "image/webp",
        ImageFormat::Pnm => "image/x-portable-anymap",
        ImageFormat::Tiff => "image/tiff",
        ImageFormat::Tga => "image/x-targa",
        ImageFormat::Dds => "image/vnd.ms-dds",
        ImageFormat::Bmp => "image/bmp",
        ImageFormat::Ico => "image/x-icon",
        ImageFormat::Hdr => "image/vnd.radiance",
        ImageFormat::OpenExr => "image/x-exr",
        ImageFormat::Farbfeld => "image/farbfeld",
        ImageFormat::Avif => "image/avif",
        ImageFormat::Qoi => "image/qoi",
        _ => "application/octet-stream",
    }
}
