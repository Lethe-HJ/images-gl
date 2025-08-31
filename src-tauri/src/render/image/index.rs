use crate::utils::time::get_time;
use image::GenericImageView;
use memmap2::MmapOptions;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::ipc::Response;

// Chunk 缓存目录
const CHUNK_CACHE_DIR: &str = "chunk_cache";

// Chunk 元数据结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChunkInfo {
    pub x: u32,       // chunk 在图片中的 X 坐标
    pub y: u32,       // chunk 在图片中的 Y 坐标
    pub width: u32,   // chunk 宽度
    pub height: u32,  // chunk 高度
    pub chunk_x: u32, // chunk 的 X 索引
    pub chunk_y: u32, // chunk 的 Y 索引
}

// 图片元数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub total_width: u32,       // 图片总宽度
    pub total_height: u32,      // 图片总高度
    pub chunk_size: u32,        // chunk 大小（正方形）
    pub chunks_x: u32,          // X 方向的 chunk 数量
    pub chunks_y: u32,          // Y 方向的 chunk 数量
    pub chunks: Vec<ChunkInfo>, // 所有 chunk 信息
}

// 注意：ChunkData 结构体已删除，现在使用零拷贝方式直接返回原始数据
// 数据格式：宽度(4字节) + 高度(4字节) + RGBA像素数据

/// 获取特定图片文件的 chunk 元数据
#[tauri::command]
pub fn get_image_metadata_for_file(file_path: String) -> Result<ImageMetadata, String> {
    println!("[RUST] 开始获取图片元数据: {}", file_path);

    // 检查文件是否存在
    if !std::path::Path::new(&file_path).exists() {
        return Err(format!("图片文件不存在: {}", file_path));
    }

    // 检查是否有这个文件对应的缓存
    if check_file_cache_exists(&file_path) {
        println!("[RUST] 发现现有缓存，从缓存加载元数据");

        // 从缓存文件加载元数据
        let metadata_filepath = Path::new(CHUNK_CACHE_DIR).join("metadata.json");
        let metadata_content = fs::read_to_string(metadata_filepath)
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

    // 使用指定文件路径进行预处理
    let metadata = preprocess_and_cache_chunks_from_path(&file_path)?;

    println!("[RUST] 预处理完成，元数据已缓存");

    Ok(metadata)
}

/// 处理用户选择的图片文件
#[tauri::command]
pub fn process_user_image(file_path: String) -> Result<ImageMetadata, String> {
    let start_time = get_time();
    println!("[RUST] 开始处理用户选择的图片: {}ms", file_path);

    // 检查文件是否存在
    if !std::path::Path::new(&file_path).exists() {
        return Err(format!("图片文件不存在: {}", file_path));
    }

    // 检查文件扩展名
    let path = std::path::Path::new(&file_path);
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
        let metadata_filepath = Path::new(CHUNK_CACHE_DIR).join("metadata.json");
        let metadata_content = fs::read_to_string(metadata_filepath)
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
    let metadata = preprocess_and_cache_chunks_from_path(&file_path)?;

    let end_time = get_time();
    println!(
        "[RUST] 用户图片处理完成: {}ms (总耗时: {}ms)",
        end_time,
        end_time - start_time
    );

    Ok(metadata)
}

/// 检查特定文件路径的 chunk 缓存是否存在
fn check_file_cache_exists(file_path: &str) -> bool {
    let cache_dir = Path::new(CHUNK_CACHE_DIR);
    if !cache_dir.exists() {
        return false;
    }

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

/// 预处理图片并缓存所有 chunks 从指定路径
fn preprocess_and_cache_chunks_from_path(file_path: &str) -> Result<ImageMetadata, String> {
    let start_time = get_time();
    println!("[RUST] 开始预处理和缓存 chunks 从路径: {}ms", file_path);

    // 图片解码优化：跳过格式检测，直接使用 PNG 解码器
    let decode_start = get_time();

    // 检查文件是否存在
    if !std::path::Path::new(file_path).exists() {
        return Err(format!(
            "图片文件不存在: {} (当前工作目录: {:?})",
            file_path,
            std::env::current_dir().unwrap_or_default()
        ));
    }

    // 直接使用 PNG 解码器，跳过格式检测
    let file = std::fs::File::open(file_path)
        .map_err(|e| format!("文件打开失败: {} (路径: {})", e, file_path))?;
    let reader = std::io::BufReader::new(file);

    // 使用 PNG 解码器，避免格式检测开销
    let decoder =
        image::codecs::png::PngDecoder::new(reader).map_err(|e| format!("PNG解码失败: {}", e))?;

    let img =
        image::DynamicImage::from_decoder(decoder).map_err(|e| format!("PNG解码失败: {}", e))?;

    let decode_end = get_time();

    println!(
        "[RUST] PNG直接解码完成: {}ms (耗时: {}ms)",
        decode_end,
        decode_end - decode_start
    );

    // 获取图片尺寸
    let (total_width, total_height) = img.dimensions();
    println!("[RUST] 图片尺寸: {}x{}", total_width, total_height);

    // 计算 chunk 信息
    let chunk_size = 4096; // 增加 chunk 大小为 4096x4096，提高传输效率
    let chunks_x = (total_width + chunk_size - 1) / chunk_size; // 向上取整
    let chunks_y = (total_height + chunk_size - 1) / chunk_size; // 向上取整

    println!(
        "[RUST] Chunk 配置: {}x{} chunks, 每个 {}x{}",
        chunks_x, chunks_y, chunk_size, chunk_size
    );

    // 创建缓存目录
    let cache_dir = Path::new(CHUNK_CACHE_DIR);
    if !cache_dir.exists() {
        fs::create_dir(cache_dir).map_err(|e| format!("创建缓存目录失败: {}", e))?;
    }

    // 生成所有 chunk 信息
    let mut chunks = Vec::new();
    for chunk_y in 0..chunks_y {
        for chunk_x in 0..chunks_x {
            let x = chunk_x * chunk_size;
            let y = chunk_y * chunk_size;
            let width = std::cmp::min(chunk_size, total_width - x);
            let height = std::cmp::min(chunk_size, total_height - y);

            let chunk_info = ChunkInfo {
                x,
                y,
                width,
                height,
                chunk_x,
                chunk_y,
            };

            chunks.push(chunk_info);
        }
    }

    println!("[RUST] 生成了 {} 个 chunk 信息，开始并行处理", chunks.len());

    // 显示并行配置信息
    let num_threads = rayon::current_num_threads();
    println!("[RUST] 并行配置：使用 {} 个线程", num_threads);

    // 将图片转换为 RGBA8 格式（只转换一次，避免每个chunk重复转换）
    let rgba_conversion_start = get_time();
    let rgba_img = img.to_rgba8();
    let rgba_conversion_end = get_time();
    println!(
        "[RUST] 图片转换为RGBA8格式完成: {}ms (耗时: {}ms)",
        rgba_conversion_end,
        rgba_conversion_end - rgba_conversion_start
    );

    // 并行处理所有 chunks 并保存为单独的文件
    let parallel_start = get_time();

    // 使用 rayon 并行处理，为每个chunk生成单独的文件
    let chunk_results: Vec<Result<(), String>> = chunks
        .par_iter()
        .map(|chunk_info| {
            process_single_chunk_parallel(&rgba_img, chunk_info, cache_dir, chunk_size)
        })
        .collect();

    let parallel_end = get_time();
    println!(
        "[RUST] 并行处理完成: {}ms (耗时: {}ms)",
        parallel_end,
        parallel_end - parallel_start
    );

    // 检查是否有错误
    let total_chunks = chunks.len();
    for (i, result) in chunk_results.iter().enumerate() {
        if let Err(e) = result {
            return Err(format!("Chunk {} 处理失败: {}", i, e));
        }
    }

    println!("[RUST] 所有 {} 个 chunks 处理成功", total_chunks);

    // 保存元数据到文件
    let metadata = ImageMetadata {
        total_width,
        total_height,
        chunk_size,
        chunks_x,
        chunks_y,
        chunks: chunks.clone(),
    };

    let metadata_json =
        serde_json::to_string(&metadata).map_err(|e| format!("序列化元数据失败: {}", e))?;

    let metadata_filepath = cache_dir.join("metadata.json");
    fs::write(&metadata_filepath, metadata_json).map_err(|e| format!("保存元数据失败: {}", e))?;

    // 保存源文件信息
    let source_info = serde_json::json!({
        "file_path": file_path,
        "total_width": total_width,
        "total_height": total_height,
        "chunk_size": chunk_size,
        "chunks_x": chunks_x,
        "chunks_y": chunks_y,
    });
    let source_info_json =
        serde_json::to_string(&source_info).map_err(|e| format!("序列化源文件信息失败: {}", e))?;
    let source_info_filepath = cache_dir.join("source_info.json");
    fs::write(&source_info_filepath, source_info_json)
        .map_err(|e| format!("保存源文件信息失败: {}", e))?;

    let end_time = get_time();
    println!(
        "[RUST] 预处理和缓存完成: {}ms (总耗时: {}ms), 共 {} 个 chunks",
        end_time,
        end_time - start_time,
        total_chunks
    );

    Ok(metadata)
}

/// 获取特定 chunk 的像素数据（零拷贝版本）
#[tauri::command]
pub fn get_image_chunk(chunk_x: u32, chunk_y: u32, file_path: String) -> Result<Response, String> {
    let start_time = get_time();
    println!(
        "[RUST] 开始获取 chunk ({}, {}) 从文件 {}: {}ms",
        chunk_x, chunk_y, file_path, start_time
    );

    // 检查特定文件的缓存是否存在
    if !check_file_cache_exists(&file_path) {
        return Err(
            "Chunk 缓存不存在，请先调用 get_image_metadata_for_file 进行预处理".to_string(),
        );
    }

    // 从缓存文件读取 chunk 数据
    let chunk_filename = format!("chunk_{}_{}.bin", chunk_x, chunk_y);
    let chunk_filepath = Path::new(CHUNK_CACHE_DIR).join(&chunk_filename);

    if !chunk_filepath.exists() {
        return Err(format!("Chunk 文件不存在: {:?}", chunk_filepath));
    }

    // 直接读取文件数据，零拷贝传输
    let chunk_data =
        fs::read(&chunk_filepath).map_err(|e| format!("读取 chunk 文件失败: {}", e))?;

    // 验证数据格式：宽度(4字节) + 高度(4字节) + 像素数据
    if chunk_data.len() < 8 {
        return Err("Chunk 文件格式错误：数据长度不足".to_string());
    }

    // 解析头部信息用于日志
    let width = u32::from_be_bytes([chunk_data[0], chunk_data[1], chunk_data[2], chunk_data[3]]);
    let height = u32::from_be_bytes([chunk_data[4], chunk_data[5], chunk_data[6], chunk_data[7]]);
    let pixels_len = chunk_data.len() - 8;

    let x = chunk_x * 2048; // chunk_size
    let y = chunk_y * 2048; // chunk_size

    println!(
        "[RUST] Chunk ({}, {}) 从缓存加载成功: 位置({}, {}), 尺寸{}x{}, 像素数据{}字节",
        chunk_x, chunk_y, x, y, width, height, pixels_len
    );

    let end_time = get_time();
    println!(
        "[RUST] Chunk ({}, {}) 零拷贝获取完成: {}ms (总耗时: {}ms)",
        chunk_x,
        chunk_y,
        end_time,
        end_time - start_time
    );

    // 零拷贝返回：直接传递原始数据，避免序列化和反序列化
    // 数据格式：宽度(4字节) + 高度(4字节) + 像素数据
    // 前端可以直接解析这个格式，无需额外的JSON序列化开销
    Ok(Response::new(chunk_data))
}

/// 清理 chunk 缓存
#[tauri::command]
pub fn clear_chunk_cache() -> Result<String, String> {
    let cache_dir = Path::new(CHUNK_CACHE_DIR);
    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir).map_err(|e| format!("清理缓存目录失败: {}", e))?;
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
        fs::read_to_string(&source_info_file).map_err(|e| format!("读取源文件信息失败: {}", e))?;

    let source_info: serde_json::Value = serde_json::from_str(&source_info_content)
        .map_err(|e| format!("解析源文件信息失败: {}", e))?;

    // 检查文件路径是否匹配
    let cached_path = source_info.get("file_path").and_then(|v| v.as_str());
    if cached_path != Some(&file_path) {
        return Ok("缓存文件与指定文件不匹配".to_string());
    }

    // 清理整个缓存目录
    fs::remove_dir_all(cache_dir).map_err(|e| format!("清理缓存目录失败: {}", e))?;
    println!("[RUST] 文件 {} 的缓存已清理", file_path);
    Ok(format!("文件 {} 的缓存已清理", file_path))
}

/// 手动触发预处理和缓存（用于测试或强制更新）
#[tauri::command]
pub fn force_preprocess_chunks(file_path: String) -> Result<ImageMetadata, String> {
    println!("[RUST] 手动触发预处理和缓存: {}", file_path);

    // 先清理现有缓存
    let _ = clear_file_cache(file_path.clone());

    // 重新预处理和缓存
    let metadata = preprocess_and_cache_chunks_from_path(&file_path)?;

    println!("[RUST] 手动预处理完成");
    Ok(metadata)
}

/// 优化的像素提取函数
fn extract_chunk_pixels_optimized(
    rgba_img: &image::RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    // 预分配内存，避免动态扩容
    let pixel_count = (width * height) as usize;
    let mut pixels = Vec::with_capacity(pixel_count * 4);

    // 直接使用 view 方法获取指定区域，避免重复转换
    let chunk_view = rgba_img.view(x, y, width, height);

    // 批量提取像素数据 - 使用更高效的访问方式
    for y_offset in 0..height {
        for x_offset in 0..width {
            let pixel = chunk_view.get_pixel(x_offset, y_offset);
            // 使用 extend_from_slice 批量添加，减少 push 调用次数
            pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], pixel[3]]);
        }
    }

    pixels
}

/// 并行处理单个 chunk 的函数
fn process_single_chunk_parallel(
    rgba_img: &image::RgbaImage,
    chunk_info: &ChunkInfo,
    cache_dir: &Path,
    _chunk_size: u32,
) -> Result<(), String> {
    let chunk_start = get_time();

    // 提取指定区域的像素数据（优化版本）
    let pixels = extract_chunk_pixels_optimized(
        rgba_img,
        chunk_info.x,
        chunk_info.y,
        chunk_info.width,
        chunk_info.height,
    );

    // 保存 chunk 到文件（使用内存映射优化）
    let chunk_filename = format!("chunk_{}_{}.bin", chunk_info.chunk_x, chunk_info.chunk_y);
    let chunk_filepath = cache_dir.join(&chunk_filename);

    // 计算chunk文件大小：宽度(4字节) + 高度(4字节) + 像素数据
    let chunk_file_size = 8 + pixels.len() as u64;

    // 创建文件并设置大小
    let chunk_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&chunk_filepath)
        .map_err(|e| {
            format!(
                "创建 chunk ({}, {}) 文件失败: {}",
                chunk_info.chunk_x, chunk_info.chunk_y, e
            )
        })?;

    // 设置文件大小
    chunk_file.set_len(chunk_file_size).map_err(|e| {
        format!(
            "设置 chunk ({}, {}) 文件大小失败: {}",
            chunk_info.chunk_x, chunk_info.chunk_y, e
        )
    })?;

    // 创建内存映射
    let mmap = unsafe {
        MmapOptions::new().map_mut(&chunk_file).map_err(|e| {
            format!(
                "创建 chunk ({}, {}) 内存映射失败: {}",
                chunk_info.chunk_x, chunk_info.chunk_y, e
            )
        })?
    };

    // 写入数据到内存映射
    let mut mmap_guard = mmap;

    // 写入头部信息
    mmap_guard[0..4].copy_from_slice(&chunk_info.width.to_be_bytes());
    mmap_guard[4..8].copy_from_slice(&chunk_info.height.to_be_bytes());

    // 写入像素数据
    mmap_guard[8..].copy_from_slice(&pixels);

    // 同步到磁盘
    mmap_guard.flush().map_err(|e| {
        format!(
            "同步 chunk ({}, {}) 到磁盘失败: {}",
            chunk_info.chunk_x, chunk_info.chunk_y, e
        )
    })?;

    let chunk_end = get_time();
    println!(
        "[RUST] Chunk ({}, {}) 内存映射处理完成: {}ms (耗时: {}ms), 像素: {}, 文件大小: {} 字节",
        chunk_info.chunk_x,
        chunk_info.chunk_y,
        chunk_end,
        chunk_end - chunk_start,
        pixels.len() / 4,
        chunk_file_size
    );

    Ok(())
}
