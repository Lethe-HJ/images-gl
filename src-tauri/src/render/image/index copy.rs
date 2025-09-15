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

/// 同步版本的 chunk 获取函数（在 rayon 线程中执行）
fn get_image_chunk_sync(chunk_x: u32, chunk_y: u32, file_path: String) -> Result<Response, String> {
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
