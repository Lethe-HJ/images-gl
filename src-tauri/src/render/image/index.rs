use crate::utils::time::get_time;
use image::GenericImageView;
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

// Chunk 数据响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ChunkData {
    pub chunk_x: u32,    // chunk 的 X 索引
    pub chunk_y: u32,    // chunk 的 Y 索引
    pub x: u32,          // chunk 在图片中的 X 坐标
    pub y: u32,          // chunk 在图片中的 Y 坐标
    pub width: u32,      // chunk 宽度
    pub height: u32,     // chunk 高度
    pub pixels: Vec<u8>, // RGBA 像素数据
}

#[tauri::command]
pub fn read_file() -> Result<Response, String> {
    let start_time = get_time();
    println!("[RUST] 开始读取图片: {}ms", start_time);

    // 使用正确的相对路径
    let file_path = "../public/tissue_hires_image.png";

    // 图片解码优化：跳过格式检测，直接使用 PNG 解码器
    let decode_start = get_time();

    // 直接使用 PNG 解码器，跳过格式检测
    let file = std::fs::File::open(file_path).map_err(|e| format!("文件打开失败: {}", e))?;
    let reader = std::io::BufReader::new(file);

    // 使用 PNG 解码器，避免格式检测开销
    let decoder = image::codecs::png::PngDecoder::new(reader)
        .map_err(|e| format!("PNG解码器创建失败: {}", e))?;

    let img =
        image::DynamicImage::from_decoder(decoder).map_err(|e| format!("PNG解码失败: {}", e))?;

    let decode_end = get_time();

    println!(
        "[RUST] PNG直接解码完成: {}ms (耗时: {}ms)",
        decode_end,
        decode_end - decode_start
    );

    // 获取图片尺寸并打印（用于调试）
    let (width, height) = img.dimensions();
    println!("[RUST] 图片尺寸: {}x{}", width, height);

    // RGBA转换优化：直接获取像素数据，避免不必要的转换
    let convert_start = get_time();

    // 检查图片是否已经是 RGBA8 格式，避免不必要的转换
    let pixels = match img {
        image::DynamicImage::ImageRgba8(rgba) => {
            println!("[RUST] 图片已经是 RGBA8 格式，直接使用");
            rgba.into_raw()
        }
        _ => {
            println!("[RUST] 图片需要转换为 RGBA8 格式");
            let rgba_img = img.to_rgba8();
            rgba_img.into_raw()
        }
    };

    let convert_end = get_time();
    println!(
        "[RUST] RGBA处理完成: {}ms (耗时: {}ms)",
        convert_end,
        convert_end - convert_start
    );

    // 创建包含尺寸信息的头部（8字节）
    let mut data = Vec::with_capacity(8 + pixels.len());
    data.extend_from_slice(&width.to_be_bytes()); // 4字节宽度
    data.extend_from_slice(&height.to_be_bytes()); // 4字节高度

    data.extend_from_slice(&pixels); // 像素数据

    let end_time = get_time();
    let data_size = data.len(); // 在移动前保存大小

    // 返回带有尺寸信息的像素数据
    let response = Ok(Response::new(data));
    println!(
        "[RUST] 图片处理完成: {}ms (总耗时: {}ms)",
        end_time,
        end_time - start_time
    ); // 总体耗时最少记录为 2190ms
    println!(
        "[RUST] 数据大小: {} 字节 (头部: 8字节, 像素: {}字节)",
        data_size,
        pixels.len()
    );
    return response;
    // 这里的返回是直接将内存区域传递给前端进行读取, 不需要进行任何序列化和反序列化, 甚至不需要拷贝
    // 经测试 这里返回数据与 ts 读取到数据耗时约 67ms
}

/// 获取图片的 chunk 元数据
#[tauri::command]
pub fn get_image_metadata() -> Result<ImageMetadata, String> {
    let start_time = get_time();
    println!("[RUST] 开始获取图片元数据: {}ms", start_time);

    // 检查缓存是否存在
    if check_chunk_cache_exists() {
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

    // 预处理并缓存所有 chunks
    let metadata = preprocess_and_cache_chunks()?;

    println!("[RUST] 预处理完成，元数据已缓存");

    Ok(metadata)
}

/// 检查 chunk 缓存是否存在
fn check_chunk_cache_exists() -> bool {
    let cache_dir = Path::new(CHUNK_CACHE_DIR);
    if !cache_dir.exists() {
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

/// 预处理图片并缓存所有 chunks
fn preprocess_and_cache_chunks() -> Result<ImageMetadata, String> {
    let start_time = get_time();
    println!("[RUST] 开始预处理和缓存 chunks: {}ms", start_time);

    // 记录优化信息
    println!("[RUST] 使用优化版本：预分配内存 + view 方法 + 批量像素提取 + 并行处理");

    // 使用正确的相对路径 - 从当前工作目录开始
    let file_path = "../public/tissue_hires_image.png";

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
    let decoder = image::codecs::png::PngDecoder::new(reader)
        .map_err(|e| format!("PNG解码器创建失败: {}", e))?;

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
    let chunk_size = 1024; // 固定 chunk 大小为 1024x1024
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

    // 并行处理所有 chunks
    let parallel_start = get_time();

    // 使用 rayon 并行处理
    let chunk_results: Vec<Result<(), String>> = chunks
        .par_iter()
        .map(|chunk_info| process_single_chunk_parallel(&img, chunk_info, &cache_dir, chunk_size))
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

    let end_time = get_time();
    println!(
        "[RUST] 预处理和缓存完成: {}ms (总耗时: {}ms), 共 {} 个 chunks",
        end_time,
        end_time - start_time,
        total_chunks
    );

    Ok(metadata)
}

/// 获取特定 chunk 的像素数据
#[tauri::command]
pub fn get_image_chunk(chunk_x: u32, chunk_y: u32) -> Result<ChunkData, String> {
    let start_time = get_time();
    println!(
        "[RUST] 开始获取 chunk ({}, {}): {}ms",
        chunk_x, chunk_y, start_time
    );

    // 检查缓存是否存在
    if !check_chunk_cache_exists() {
        return Err("Chunk 缓存不存在，请先调用 get_image_metadata 进行预处理".to_string());
    }

    // 从缓存文件读取 chunk 数据
    let chunk_filename = format!("chunk_{}_{}.bin", chunk_x, chunk_y);
    let chunk_filepath = Path::new(CHUNK_CACHE_DIR).join(&chunk_filename);

    if !chunk_filepath.exists() {
        return Err(format!("Chunk 文件不存在: {:?}", chunk_filepath));
    }

    let chunk_data =
        fs::read(&chunk_filepath).map_err(|e| format!("读取 chunk 文件失败: {}", e))?;

    // 解析 chunk 数据：宽度(4字节) + 高度(4字节) + 像素数据
    if chunk_data.len() < 8 {
        return Err("Chunk 文件格式错误：数据长度不足".to_string());
    }

    let width = u32::from_be_bytes([chunk_data[0], chunk_data[1], chunk_data[2], chunk_data[3]]);
    let height = u32::from_be_bytes([chunk_data[4], chunk_data[5], chunk_data[6], chunk_data[7]]);
    let pixels = chunk_data[8..].to_vec();

    let x = chunk_x * 1024; // chunk_size
    let y = chunk_y * 1024; // chunk_size

    println!(
        "[RUST] Chunk ({}, {}) 从缓存加载成功: 位置({}, {}), 尺寸{}x{}, 像素数据{}字节",
        chunk_x,
        chunk_y,
        x,
        y,
        width,
        height,
        pixels.len()
    );

    let end_time = get_time();
    println!(
        "[RUST] Chunk ({}, {}) 获取完成: {}ms (总耗时: {}ms)",
        chunk_x,
        chunk_y,
        end_time,
        end_time - start_time
    );

    Ok(ChunkData {
        chunk_x,
        chunk_y,
        x,
        y,
        width,
        height,
        pixels,
    })
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

/// 手动触发预处理和缓存（用于测试或强制更新）
#[tauri::command]
pub fn force_preprocess_chunks() -> Result<ImageMetadata, String> {
    println!("[RUST] 手动触发预处理和缓存");

    // 先清理现有缓存
    let _ = clear_chunk_cache();

    // 重新预处理和缓存
    let metadata = preprocess_and_cache_chunks()?;

    println!("[RUST] 手动预处理完成");
    Ok(metadata)
}

/// 优化的像素提取函数
fn extract_chunk_pixels_optimized(
    img: &image::DynamicImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    // 预分配内存，避免动态扩容
    let pixel_count = (width * height) as usize;
    let mut pixels = Vec::with_capacity(pixel_count * 4);

    // 将图片转换为 RGBA8 格式（只转换一次）
    let rgba_img = img.to_rgba8();

    // 使用 view 方法直接获取指定区域，避免逐像素访问
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
    img: &image::DynamicImage,
    chunk_info: &ChunkInfo,
    cache_dir: &Path,
    chunk_size: u32,
) -> Result<(), String> {
    let chunk_start = get_time();

    // 提取指定区域的像素数据（优化版本）
    let pixels = extract_chunk_pixels_optimized(
        img,
        chunk_info.x,
        chunk_info.y,
        chunk_info.width,
        chunk_info.height,
    );

    // 保存 chunk 到文件
    let chunk_filename = format!("chunk_{}_{}.bin", chunk_info.chunk_x, chunk_info.chunk_y);
    let chunk_filepath = cache_dir.join(&chunk_filename);

    // 写入 chunk 数据：宽度(4字节) + 高度(4字节) + 像素数据
    // 预分配内存，避免动态扩容
    let mut chunk_data = Vec::with_capacity(8 + pixels.len());
    chunk_data.extend_from_slice(&chunk_info.width.to_be_bytes());
    chunk_data.extend_from_slice(&chunk_info.height.to_be_bytes());
    chunk_data.extend_from_slice(&pixels);

    fs::write(&chunk_filepath, chunk_data).map_err(|e| {
        format!(
            "保存 chunk ({}, {}) 失败: {}",
            chunk_info.chunk_x, chunk_info.chunk_y, e
        )
    })?;

    let chunk_end = get_time();
    println!(
        "[RUST] Chunk ({}, {}) 并行处理完成: {}ms (耗时: {}ms), 像素: {}",
        chunk_info.chunk_x,
        chunk_info.chunk_y,
        chunk_end,
        chunk_end - chunk_start,
        pixels.len() / 4
    );

    Ok(())
}
