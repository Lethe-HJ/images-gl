use crate::utils::time::get_time;
use image::GenericImageView;
use rayon::prelude::*;
use serde_json;
use std::cmp;
use std::env;
use std::fs;
use std::io;
use std::path::Path;

use super::cache::check_file_cache_exists;
use super::chunk_processing::process_single_chunk_parallel;
use super::config::{CHUNK_CACHE_DIR, CHUNK_SIZE_X, CHUNK_SIZE_Y};
use super::types::{ChunkInfo, ImageMetadata};

/// 获取特定图片文件的 chunk 元数据
/// # Arguments
/// * `file_path` - 图片文件路径
/// # Returns
/// * `Result<ImageMetadata, String>` - 图片元数据或错误信息
#[tauri::command] // 这个宏 声明了这个函数是 tauri command，表示这个函数可以被前端调用
pub fn get_image_metadata_for_file(file_path: String) -> Result<ImageMetadata, String> {
    println!("[RUST] 开始获取图片元数据: {}", file_path);

    // 检查文件是否存在
    if !Path::new(&file_path).exists() {
        return Err(format!("图片文件不存在: {}", file_path));
    }

    // 检查是否有这个文件对应的缓存
    if check_file_cache_exists(&file_path) {
        println!("[RUST] 发现现有缓存，从缓存加载元数据");

        // 从缓存文件加载元数据 缓存文件是json格式 位于缓存目录下 文件名为metadata.json
        // TODO 这个地方 缓存文件是统一的一个 当已经被缓存过的文件多了之后 这个文件会变得很大 需要优化 最好是每个图片对应的metadata.json都不一样
        let metadata_filepath = Path::new(CHUNK_CACHE_DIR).join("metadata.json");
        // 读取缓存文件成字符串
        let metadata_content = fs::read_to_string(metadata_filepath)
            .map_err(|e| format!("读取缓存元数据失败: {}", e))?;
        // 将字符串反序列化为json
        let metadata: ImageMetadata = serde_json::from_str(&metadata_content)
            .map_err(|e| format!("解析缓存元数据失败: {}", e))?;

        println!(
            "[RUST] 从缓存加载元数据成功: {}x{}, 共 {} 个 chunks",
            metadata.total_width,
            metadata.total_height,
            metadata.chunks.len()
        );
        // 给前端返回元数据
        return Ok(metadata);
    }

    println!("[RUST] 缓存不存在，开始预处理和缓存 chunks");

    // 使用指定文件路径进行预处理
    let metadata = preprocess_and_cache_chunks(&file_path)?;

    println!("[RUST] 预处理完成，元数据已缓存");

    Ok(metadata)
}

/// 预处理图片并缓存所有 chunks
/// # Arguments
/// * `file_path` - 图片文件路径
/// # Returns
/// * `Result<ImageMetadata, String>` - 图片元数据或错误信息
pub fn preprocess_and_cache_chunks(file_path: &str) -> Result<ImageMetadata, String> {
    let start_time = get_time();
    println!("[RUST] 开始预处理和缓存 chunks 从路径: {}ms", file_path);

    let decode_start = get_time();

    // 检查文件是否存在
    if !Path::new(file_path).exists() {
        return Err(format!(
            "图片文件不存在: {} (当前工作目录: {:?})",
            file_path,
            env::current_dir().unwrap_or_default()
        ));
    }

    let file = fs::File::open(file_path)
        .map_err(|e| format!("文件打开失败: {} (路径: {})", e, file_path))?;
    let reader = io::BufReader::new(file);

    // TODO 这里后续还会支持更加适合lod的图片格式 tiff
    // 创建解码器
    let decoder =
        image::codecs::png::PngDecoder::new(reader).map_err(|e| format!("PNG解码失败: {}", e))?;
    // 从解码器中获取动态image对象
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

    // NOTE rust中 u32类型的除法 会向下取整

    // 下面推导一共需要多少行多少列chunk
    // 先来符合直觉的推导思路
    //
    // 1. 先考虑特殊情况 图片宽度不是chunk_size的整数倍时 需要使用更多的chunk才能完全囊括
    // -----------------
    // |      |      |  .   |
    // |      |      |  .   |
    // -----------------
    // 如图所示 实际图片宽度只有两个多chunk的宽度 但是仍然需要使用三个chunk才能完全囊括
    // 此时表达式应该为 total_width / chunk_size + 1
    //
    // 2. 再考虑一般情况 图片宽度是chunk_size的整数倍时
    // ---------------
    // |      |      |
    // |      |      |
    // ---------------
    // 此时表达式应该为 total_width / chunk_size
    // 但是这样一来就没办法兼容情况1了 考虑将total_width减去1 这个时候情况2就转换成了情况1
    // 如果本身就是在情况1的状况下total_width减去1不影响结果
    //
    // 因此 更加通用的表达式为 (total_width - 1) / chunk_size + 1 与代码里面的表达式等效
    //
    // 再考虑更加数学的推导思路
    // total_width chunk_size col_count
    //    401         200       3
    //    400         200       2   特殊情况 刚好整除
    //    399         200       2
    //     0          200       0
    // 不考虑特殊情况的情况下 可归纳为 total_width / chunk_size + 1
    // 对于特殊情况 考虑将total_width减去1 这个时候情况2就转换成了情况1
    // 如果本身就是在情况1的状况下total_width减去1不影响结果
    // 因此 更加通用的表达式为 (total_width - 1) / chunk_size + 1 与代码里面的表达式等效

    let col_count = (total_width + CHUNK_SIZE_X - 1) / CHUNK_SIZE_X;
    let row_count = (total_height + CHUNK_SIZE_Y - 1) / CHUNK_SIZE_Y;

    println!(
        "[RUST] Chunk 配置: {}x{} chunks, 每个 {}x{}",
        col_count, row_count, CHUNK_SIZE_X, CHUNK_SIZE_Y
    );

    // 创建缓存目录
    let cache_dir = Path::new(CHUNK_CACHE_DIR);
    if !cache_dir.exists() {
        fs::create_dir(cache_dir).map_err(|e| format!("创建缓存目录失败: {}", e))?;
    }

    // NOTE
    // Vec 动态数组
    // 特点: 连续存储 动态大小 自动扩容
    // 创建方式 Vec::new() 或者 Vec::with_capacity(capacity)

    // NOTE
    // unwrap 是 Rust 中的一个宏，用于将 Result 类型转换为 Option 类型
    // 如果 Result 类型是 Ok，则返回 Ok 中的值
    // 如果 Result 类型是 Err，则 panic

    // 生成所有 chunk 信息
    let chunks_count = usize::try_from(col_count * row_count).unwrap();
    let mut chunks = Vec::with_capacity(chunks_count);
    for chunk_y in 0..row_count {
        for chunk_x in 0..col_count {
            let x = chunk_x * CHUNK_SIZE_X;
            let y = chunk_y * CHUNK_SIZE_Y;
            let width = cmp::min(CHUNK_SIZE_X, total_width - x);
            let height = cmp::min(CHUNK_SIZE_Y, total_height - y);

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
        .par_iter() // 将chunks迭代器转换为并行迭代器
        .map(|chunk_info| process_single_chunk_parallel(&rgba_img, chunk_info, cache_dir))
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
        chunk_size_x: CHUNK_SIZE_X,
        chunk_size_y: CHUNK_SIZE_Y,
        col_count,
        row_count,
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
        "chunk_size_x": CHUNK_SIZE_X,
        "chunk_size_y": CHUNK_SIZE_Y,
        "col_count": col_count,
        "row_count": row_count,
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
