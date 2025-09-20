use serde_json;
use std::fs;
use std::path::Path;

use super::config::CHUNK_CACHE_DIR;

/// 检查特定文件路径的 chunk 缓存是否存在
/// # Arguments
/// * `file_path` - 图片文件路径
/// # Returns
/// * `bool` - 是否存在缓存
pub fn check_file_cache_exists(file_path: &str) -> bool {
    let cache_dir = Path::new(CHUNK_CACHE_DIR);
    if !cache_dir.exists() {
        return false;
    }

    // TODO 这个地方 源文件信息文件是统一的一个 当已经被缓存过的文件多了之后 这个文件会变得很大 需要优化 最好是每个图片对应的source_info.json都不一样
    // 检查源文件信息文件是否存在
    let source_info_file = cache_dir.join("source_info.json");
    if !source_info_file.exists() {
        return false;
    }

    // 读取源文件信息
    let source_info_content = match fs::read_to_string(&source_info_file) {
        Ok(content) => content,
        Err(_) => return false,
    };

    let source_info: serde_json::Value = match serde_json::from_str(&source_info_content) {
        Ok(info) => info,
        Err(_) => return false,
    };

    // 检查文件路径是否匹配
    let cached_path = source_info.get("file_path").and_then(|v| v.as_str());
    if cached_path != Some(file_path) {
        return false;
    }

    // 检查元数据文件是否存在
    let metadata_file = cache_dir.join("metadata.json");
    if !metadata_file.exists() {
        return false;
    }

    // 检查是否有 chunk 文件
    if let Ok(entries) = fs::read_dir(cache_dir) {
        let chunk_files: Vec<_> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("chunk_"))
            .collect();

        return !chunk_files.is_empty();
    }

    false
}

/// 清理 chunk 缓存
#[tauri::command]
pub fn clear_chunk_cache() -> Result<String, String> {
    let cache_dir = Path::new(CHUNK_CACHE_DIR);
    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir).map_err(|e| format!("清理缓存目录失败: {e}"))?;
        println!("[RUST] Chunk 缓存已清理");
        Ok("Chunk 缓存已清理".to_string())
    } else {
        Ok("Chunk 缓存不存在".to_string())
    }
}

/// 清理特定文件的 chunk 缓存
#[tauri::command]
pub fn clear_file_cache(file_path: String) -> Result<String, String> {
    let cache_dir = Path::new(CHUNK_CACHE_DIR);
    if !cache_dir.exists() {
        return Ok("缓存目录不存在".to_string());
    }

    // 检查源文件信息文件是否存在
    let source_info_file = cache_dir.join("source_info.json");
    if !source_info_file.exists() {
        return Ok("源文件信息不存在".to_string());
    }

    // 读取源文件信息
    let source_info_content =
        fs::read_to_string(&source_info_file).map_err(|e| format!("读取源文件信息失败: {e}"))?;

    let source_info: serde_json::Value = serde_json::from_str(&source_info_content)
        .map_err(|e| format!("解析源文件信息失败: {e}"))?;

    // 检查文件路径是否匹配
    let cached_path = source_info.get("file_path").and_then(|v| v.as_str());
    if cached_path != Some(&file_path) {
        return Ok("缓存文件与指定文件不匹配".to_string());
    }

    // 清理整个缓存目录
    fs::remove_dir_all(cache_dir).map_err(|e| format!("清理缓存目录失败: {e}"))?;
    println!("[RUST] 文件 {file_path} 的缓存已清理");
    Ok(format!("文件 {file_path} 的缓存已清理"))
}
