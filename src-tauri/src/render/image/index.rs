use crate::utils::time::get_time;
use image::GenericImageView;
use memmap2::MmapOptions;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::OnceLock;
use std::thread;
use tauri::ipc::Response;

/**
 * NOTE 生命周期标注
 * 'static 表示这个生命周期与程序运行时间相同 其中'字符用于标记生命周期
 */

// 全局线程池，避免重复创建
/*
 * OnceLock 类型来确保线程池只被初始化一次
 * OnceLock 是 Rust 标准库提供的线程安全的一次性初始化容器 它存储的是 rayon 库的 ThreadPool 类型
 *
 * [语法]: static用于定义静态变量
 */
static THREAD_POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();

// 获取全局线程池
/*
 * 返回一个静态生命周期的线程池引用
 */
fn get_thread_pool() -> &'static rayon::ThreadPool {
    /*
     * NOTE: 闭包
     * || { ... } - 不带参数的闭包
     * |x| { ... } - 单参数闭包
     * |x, y| { ... } - 多参数闭包
     * 其中{}里面的内容如果是单行代码，则可以省略大括号
     * 下面的|n| n.get() 相当于 |n| { n.get() }
     */
    /*
     * get_or_init 方法确保线程池只被初始化一次
     * 如果线程池已经存在，直接返回现有的线程池
     * 如果不存在，则执行闭包中的初始化代码
     * 如果获取失败，默认使用 4 个核心
     */
    THREAD_POOL.get_or_init(|| {
        let num_cpu = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        // 设置线程数为 CPU 核心数的 2 倍
        // 但最大不超过 8 个线程
        // 这是一个经验值，适用于 I/O 密集型任务
        // 如果线程数太多 会导致过多的上下文切换

        // NOTE - src/render/why.md 为什么过多的线程会导致过多的上下文切换 仔细解释一下其中的原理?
        let optimal_threads = (num_cpu * 2).min(8);

        /*
         * NOTE 宏
         * 在 Rust 中以 ! 结尾的都是宏
         * 宏是一种代码生成器，在编译时展开
         * 可以生成重复的代码，减少手动编写
         * 比普通函数更灵活，可以接受可变数量的参数
         */

        println!(
            "[RUST] 系统 CPU 核心数: {}, 设置线程池大小: {}",
            num_cpu, optimal_threads
        );

        /*
         * 使用 rayon 库的 ThreadPoolBuilder 创建线程池
         * 设置线程数为之前计算的最优值
         * build() 构建线程池
         * unwrap() 在构建失败时会导致程序崩溃（在这种情况下是可以接受的，因为线程池是程序运行的基础设施）
         */
        rayon::ThreadPoolBuilder::new()
            .num_threads(optimal_threads)
            .build()
            .unwrap()
    })
}

/*
 * NOTE &str: 字符串切片类型
 * &str: 字符串切片类型，是一个不可变的字符串引用
 */

// Chunk 缓存目录
const CHUNK_CACHE_DIR: &str = "chunk_cache";

/*
 * NOTE 派生宏
 * #[derive(...)] 派生宏为结构体自动实现了多个特性：
 * Debug: 用于调试输出
 * Serialize, Deserialize: 支持 JSON 序列化和反序列化
 * Clone: 允许创建结构体的深拷贝
 */

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
    pub chunk_size_x: u32,      // chunk 大小 X 方向（正方形）
    pub chunk_size_y: u32,      // chunk 大小（正方形）
    pub col_count: u32,         // X 方向的 chunk 数量
    pub row_count: u32,         // Y 方向的 chunk 数量
    pub chunks: Vec<ChunkInfo>, // 所有 chunk 信息
}

/**
 * NOTE 文档注释与普通注释
 * 在rust中 /// 作为文档注释 会出现在rustdoc生成的文档中
 * 而 // 作为普通注释 不会出现在rustdoc生成的文档中
 * 可以通过 rust doc 命令来生成文档
 */

/**
 * NOTE Result
 * Result<ImageMetadata, String>
 * Result 是 Rust 标准库提供的泛型类型，用于表示可能成功或失败的操作
 * ImageMetadata 是图片元数据结构
 * String 是错误信息类型
 *
 * Result 类型用于处理可能出现错误的情况
 * 如果操作成功，返回 Ok(ImageMetadata)
 * 如果操作失败，返回 Err(String)
 */

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

    /*
     * NOTE rust重要知识点 所有权  (硬核来袭🙀🙀🙀)
     * 1. ownership 所有权
     *      rust中一个变量只能被一个环境所拥有,这个环境可以是变量,函数参数,函数返回值,结构体字段等
     *      所有权转移的过程称之为 ”move“ ”移动“
     * 2. move 移动
     *      a. ”move“会将所有权进行传递 传递之后 原来的变量将不再有效
     *          如果想要继续使用原来的变量 需要使用 ”clone“ ”克隆“
     *      b. ”move“时 栈上的数据会被复制
     *         而堆上的数据会进行所有权转移 这部分数据不会进行拷贝
     *      c. 变量的信息哪些位于栈上哪些位于堆上的数据:
     *            以String类型举例 其指针、长度、容量等信息存放于栈上
     *            实际的字符串内容位于堆上
     *      d. 什么时候会发生 move
     *            将一个变量赋值给另一个变量 比如使用“=”进行复制或者函数参数传递之类(还有很多...)
     * 3. 引用 “&”
     *      a. 引用的本质就是“指针”
     *      b. 引用不会拥有所有权 不会发生所有权转移
     * 4. 克隆 “clone”
     *      a. 克隆会创建一个完全独立的新副本
     *      b. 赋值时使用克隆 可以避免原来的值发生所有权转移 从而不可用
     *      c. 克隆会消耗更多内存和计算资源
     */

    // NOTE 在Rust中 常用“&” 来传递参数 避免所有权转移

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

/// 检查特定文件路径的 chunk 缓存是否存在
/// # Arguments
/// * `file_path` - 图片文件路径
/// # Returns
/// * `bool` - 是否存在缓存
fn check_file_cache_exists(file_path: &str) -> bool {
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

// TODO 这个chunk可能不是最优的 后续需要进行实验 或者 这个尺寸应该是实时计算后确定的
const CHUNK_SIZE_X: u32 = 4096;
const CHUNK_SIZE_Y: u32 = 4096;
// 单个chunk的内存大小应该为 4096 * 4096 * 4 = 67,108,864 字节
// 约等于 67MB

/// 预处理图片并缓存所有 chunks
/// # Arguments
/// * `file_path` - 图片文件路径
/// # Returns
/// * `Result<ImageMetadata, String>` - 图片元数据或错误信息
fn preprocess_and_cache_chunks(file_path: &str) -> Result<ImageMetadata, String> {
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

/// 并行处理单个 chunk 的函数
/// # Arguments
/// * `rgba_img` - 图片 RGBA8 格式
/// * `chunk_info` - chunk 信息
/// * `cache_dir` - 缓存目录
/// # Returns
/// * `Result<(), String>` - 是否成功
fn process_single_chunk_parallel(
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
fn extract_chunk_pixels(
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

// TODO 下面这个上面的extract_chunk_pixels函数的simd油优化版本 未经测试
// use std::simd::*; // 需要使用 nightly Rust

// fn extract_chunk_pixels_simd(
//     rgba_img: &image::RgbaImage,
//     x: u32,
//     y: u32,
//     width: u32,
//     height: u32,
// ) -> Vec<u8> {
//     let pixel_count = (width * height) as usize;
//     let mut pixels = Vec::with_capacity(pixel_count * 4);
//     let chunk_view = rgba_img.view(x, y, width, height);

//     // 使用 SIMD 一次处理多个像素
//     let simd_width = width / 4 * 4; // 确保能被4整除的部分

//     for y_offset in 0..height {
//         let mut x_offset = 0;

//         // SIMD 处理主循环
//         while x_offset < simd_width {
//             // 一次加载 4 个像素（16字节）
//             // 使用 SIMD 指令并行处理
//             x_offset += 4;
//         }

//         // 处理剩余的像素
//         while x_offset < width {
//             let pixel = chunk_view.get_pixel(x_offset, y_offset);
//             pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], pixel[3]]);
//             x_offset += 1;
//         }
//     }
//     pixels
// }
