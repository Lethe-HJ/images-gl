use crate::utils::time::get_time;
use std::path::Path;
use tauri::ipc::Response;

use super::cache::{check_file_cache_exists, clear_file_cache};
use super::chunk_processing::get_image_chunk_sync;
use super::config::get_thread_pool;
use super::preprocessing::preprocess_and_cache_chunks;
use super::types::ImageMetadata;

/// 处理用户选择的图片文件
#[tauri::command]
pub fn process_user_image(file_path: String) -> Result<ImageMetadata, String> {
    let start_time = get_time();
    println!("[RUST] 开始处理用户选择的图片: {}ms", file_path);

    // 检查文件是否存在
    if !Path::new(&file_path).exists() {
        return Err(format!("图片文件不存在: {}", file_path));
    }

    // 检查文件扩展名
    let path = Path::new(&file_path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "bmp" | "tiff" | "webp"
    ) {
        return Err(format!(
            "不支持的图片格式: {}. 支持的格式: PNG, JPG, JPEG, BMP, TIFF, WEBP",
            extension
        ));
    }

    // 先检查是否有这个文件对应的缓存
    if check_file_cache_exists(&file_path) {
        println!("[RUST] 发现现有缓存，从缓存加载元数据");

        // 从缓存文件加载元数据
        let metadata_filepath = std::path::Path::new("chunk_cache").join("metadata.json");
        let metadata_content = std::fs::read_to_string(metadata_filepath)
            .map_err(|e| format!("读取缓存元数据失败: {}", e))?;

        let metadata: ImageMetadata = serde_json::from_str(&metadata_content)
            .map_err(|e| format!("解析缓存元数据失败: {}", e))?;

        println!(
            "[RUST] 从缓存加载元数据成功: {}x{}, 共 {} 个 chunks",
            metadata.total_width,
            metadata.total_height,
            metadata.chunks.len()
        );

        return Ok(metadata);
    }

    println!("[RUST] 缓存不存在，开始预处理和缓存 chunks");

    // 使用用户选择的文件路径进行预处理
    let metadata = preprocess_and_cache_chunks(&file_path)?;

    let end_time = get_time();
    println!(
        "[RUST] 用户图片处理完成: {}ms (总耗时: {}ms)",
        end_time,
        end_time - start_time
    );

    Ok(metadata)
}

/// 获取特定 chunk 的像素数据（零拷贝版本，支持并行执行）
#[tauri::command]
pub fn get_image_chunk(chunk_x: u32, chunk_y: u32, file_path: String) -> Result<Response, String> {
    // 使用全局线程池让每个请求并行执行
    // 这样前端多个 invoke 调用时，Rust 端可以并行处理
    let result = get_thread_pool().install(|| get_image_chunk_sync(chunk_x, chunk_y, file_path));

    // 零拷贝返回：直接传递原始数据，避免序列化和反序列化
    // 数据格式：宽度(4字节) + 高度(4字节) + 像素数据
    // 前端可以直接解析这个格式，无需额外的JSON序列化开销
    result
}

/// 手动触发预处理和缓存（用于测试或强制更新）
#[tauri::command]
pub fn force_preprocess_chunks(file_path: String) -> Result<ImageMetadata, String> {
    println!("[RUST] 手动触发预处理和缓存: {}", file_path);

    // 先清理现有缓存
    let _ = clear_file_cache(file_path.clone());

    // 重新预处理和缓存
    let metadata = preprocess_and_cache_chunks(&file_path)?;

    println!("[RUST] 手动预处理完成");
    Ok(metadata)
}
