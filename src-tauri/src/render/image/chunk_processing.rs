use crate::utils::time::get_time;
use image::GenericImageView;
use memmap2::MmapOptions;
use std::fs;
use std::path::Path;
use std::thread;
use tauri::ipc::Response;

use super::cache::check_file_cache_exists;
use super::config::CHUNK_CACHE_DIR;
use super::types::ChunkInfo;

/// 并行处理单个 chunk 的函数
/// # Arguments
/// * `rgba_img` - 图片 RGBA8 格式
/// * `chunk_info` - chunk 信息
/// * `cache_dir` - 缓存目录
/// # Returns
/// * `Result<(), String>` - 是否成功
pub fn process_single_chunk_parallel(
    rgba_img: &image::RgbaImage,
    chunk_info: &ChunkInfo,
    cache_dir: &Path,
) -> Result<(), String> {
    let chunk_start = get_time();

    // 提取指定区域的像素数据
    let pixels = extract_chunk_pixels(
        rgba_img,
        chunk_info.x,
        chunk_info.y,
        chunk_info.width,
        chunk_info.height,
    );

    // TODO 这里可以维护一个像素内存池
    // 一来可以避免频繁的内存分配和释放
    // 二来前端初始访问图片的chunk时, 可以直接从内存中读取并返回, 而不需要从缓存的图片chunk文件中读取

    // NOTE
    // 内存映射文件是一种在虚拟内存和文件系统之间建立映射关系的机制。
    // 它创建一个内存区域，直接与文件的内容关联
    // 优点:
    // 1. 减少内存占用
    // 2. 提高文件读写速度
    // 3. 减少文件系统调用
    // 4. 提高文件系统性能
    // 5. 提高文件系统稳定性
    // 6. 双向映射, 既可以内存映射到文件, 也可以文件映射到内存

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

/// 像素提取函数
/// # Arguments
/// * `rgba_img` - 图片 RGBA8 格式
/// * `x` - chunk 的 X 坐标
/// * `y` - chunk 的 Y 坐标
/// * `width` - chunk 的宽度
/// * `height` - chunk 的高度
/// # Returns
/// * `Vec<u8>` - 像素数据
pub fn extract_chunk_pixels(
    rgba_img: &image::RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    // 预分配内存，避免动态扩容
    let pixel_count = (width * height) as usize;
    // rgba 需要4个字节
    let mut pixels = Vec::with_capacity(pixel_count * 4);

    // 创建图片指定区域的视图 避免重复转换
    let chunk_view = rgba_img.view(x, y, width, height);

    // 批量提取像素数据 - 使用更高效的访问方式
    for y_offset in 0..height {
        for x_offset in 0..width {
            let pixel = chunk_view.get_pixel(x_offset, y_offset);
            // 使用 extend_from_slice 批量添加，减少 push 调用次数
            // 一次添加一行
            pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], pixel[3]]);
        }
    }

    pixels
}

/// 同步版本的 chunk 获取函数（在 rayon 线程中执行）
pub fn get_image_chunk_sync(
    chunk_x: u32,
    chunk_y: u32,
    file_path: String,
) -> Result<Response, String> {
    let start_time = get_time();
    println!(
        "[RUST] 开始获取 chunk ({}, {}) 从文件 {}: {}ms (线程: {:?})",
        chunk_x,
        chunk_y,
        file_path,
        start_time,
        thread::current().id()
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

    let x = chunk_x * 2048;
    let y = chunk_y * 2048;

    println!(
        "[RUST] Chunk ({}, {}) 从缓存加载成功: 位置({}, {}), 尺寸{}x{}, 像素数据{}字节 (线程: {:?})",
        chunk_x, chunk_y, x, y, width, height, pixels_len, thread::current().id()
    );

    let end_time = get_time();
    let processing_time = end_time - start_time;

    println!(
        "[RUST] Chunk ({}, {}) 零拷贝获取完成: {}ms (总耗时: {}ms) (线程: {:?})",
        chunk_x,
        chunk_y,
        end_time,
        processing_time,
        thread::current().id()
    );

    // 零拷贝返回：直接传递原始数据，避免序列化和反序列化
    // 数据格式：宽度(4字节) + 高度(4字节) + 像素数据
    // 前端可以直接解析这个格式，无需额外的JSON序列化开销
    Ok(Response::new(chunk_data))
}
