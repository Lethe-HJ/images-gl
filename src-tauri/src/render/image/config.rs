use std::sync::OnceLock;
use std::thread;

// Chunk 缓存目录
pub const CHUNK_CACHE_DIR: &str = "chunk_cache";

// TODO 这个chunk可能不是最优的 后续需要进行实验 或者 这个尺寸应该是实时计算后确定的
pub const CHUNK_SIZE_X: u32 = 4096;
pub const CHUNK_SIZE_Y: u32 = 4096;
// 单个chunk的内存大小应该为 4096 * 4096 * 4 = 67,108,864 字节
// 约等于 67MB

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
pub fn get_thread_pool() -> &'static rayon::ThreadPool {
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

        println!("[RUST] 系统 CPU 核心数: {num_cpu}, 设置线程池大小: {optimal_threads}");

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
