// Chunk 状态管理

// Chunk 状态枚举
export enum ChunkStatus {
  UNREQUESTED = "unrequested", // 未请求
  REQUESTING = "requesting", // 请求中
  IN_CPU = "in_cpu", // 处于 CPU 中（已接收，未上传 GPU）
  IN_GPU = "in_gpu", // 处于 GPU 中（已上传到 WebGL）
  ERROR = "error", // 请求失败
}

// Chunk 信息接口
export interface ChunkInfo {
  x: number; // chunk 在图片中的 X 坐标
  y: number; // chunk 在图片中的 Y 坐标
  width: number; // chunk 宽度
  height: number; // chunk 高度
  chunk_x: number; // chunk 的 X 索引
  chunk_y: number; // chunk 的 Y 索引
}

// 图片元数据接口
export interface ImageMetadata {
  total_width: number; // 图片总宽度
  total_height: number; // 图片总高度
  chunk_size: number; // chunk 大小（正方形）
  chunks_x: number; // X 方向的 chunk 数量
  chunks_y: number; // Y 方向的 chunk 数量
  chunks: ChunkInfo[]; // 所有 chunk 信息
}

// 注意：ChunkData 接口已删除，现在直接处理二进制数据
// 数据格式：宽度(4字节) + 高度(4字节) + RGBA像素数据

// 单个 Chunk 类
export class ImageChunk {
  public id: string; // 唯一标识 (x, y)
  public x: number; // 在图片中的 X 坐标
  public y: number; // 在图片中的 Y 坐标
  public width: number; // chunk 宽度
  public height: number; // chunk 高度
  public chunk_x: number; // chunk 的 X 索引
  public chunk_y: number; // chunk 的 Y 索引
  public status: ChunkStatus; // 当前状态
  public data?: Uint8Array; // 像素数据
  public texture?: WebGLTexture; // WebGL 纹理
  public lastAccessed: number; // 最后访问时间（用于 LRU 策略）

  constructor(chunkInfo: ChunkInfo) {
    this.id = `${chunkInfo.chunk_x}_${chunkInfo.chunk_y}`;
    this.x = chunkInfo.x;
    this.y = chunkInfo.y;
    this.width = chunkInfo.width;
    this.height = chunkInfo.height;
    this.chunk_x = chunkInfo.chunk_x;
    this.chunk_y = chunkInfo.chunk_y;
    this.status = ChunkStatus.UNREQUESTED;
    this.lastAccessed = Date.now();
  }

  // 更新状态
  public updateStatus(status: ChunkStatus): void {
    this.status = status;
    if (status === ChunkStatus.IN_CPU || status === ChunkStatus.IN_GPU) {
      this.lastAccessed = Date.now();
    }
  }

  // 设置像素数据
  public setData(data: Uint8Array): void {
    this.data = data;
    this.updateStatus(ChunkStatus.IN_CPU);
  }

  // 设置纹理
  public setTexture(texture: WebGLTexture): void {
    this.texture = texture;
    this.updateStatus(ChunkStatus.IN_GPU);
  }

  // 清理资源
  public cleanup(gl: WebGL2RenderingContext): void {
    if (this.texture) {
      gl.deleteTexture(this.texture);
      this.texture = undefined;
    }
    this.data = undefined;
    this.status = ChunkStatus.UNREQUESTED;
  }
}

// Chunk 管理器类
export class ChunkManager {
  private chunks: Map<string, ImageChunk> = new Map();
  private metadata?: ImageMetadata;
  private gl?: WebGL2RenderingContext;
  private maxConcurrentRequests = 3; // 最大并发请求数
  private requestQueue: string[] = []; // 请求队列
  private activeRequests = 0; // 当前活跃请求数
  private onChunkReady?: (chunk: ImageChunk) => void; // 新增：chunk 就绪回调
  private currentFilePath?: string; // 当前处理的文件路径

  constructor() {
    console.log("[CHUNK_MANAGER] 初始化完成");
  }

  // 设置 WebGL 上下文
  public setWebGLContext(gl: WebGL2RenderingContext): void {
    this.gl = gl;
  }

  // 设置 chunk 就绪回调
  public setOnChunkReady(callback: (chunk: ImageChunk) => void): void {
    this.onChunkReady = callback;
    console.log("[CHUNK_MANAGER] Chunk 就绪回调已设置");
  }

  // 初始化 chunk 信息（从已有元数据）
  public async initializeChunksFromMetadata(
    filePath: string,
    metadata: ImageMetadata
  ): Promise<void> {
    try {
      console.log("[CHUNK_MANAGER] 从已有元数据初始化 chunks...", filePath);

      // 保存当前文件路径
      this.currentFilePath = filePath;

      // 直接使用传入的元数据
      this.metadata = metadata;

      console.log(
        `[CHUNK_MANAGER] 元数据设置成功: ${this.metadata.total_width}x${this.metadata.total_height}, 共 ${this.metadata.chunks.length} 个 chunks`
      );

      // 创建所有 chunk 对象
      this.metadata.chunks.forEach((chunkInfo) => {
        const chunk = new ImageChunk(chunkInfo);
        this.chunks.set(chunk.id, chunk);
      });

      console.log("[CHUNK_MANAGER] Chunk 初始化完成");
    } catch (error) {
      console.error("[CHUNK_MANAGER] 初始化失败:", error);
      throw error;
    }
  }

  // 初始化 chunk 信息（从后端获取元数据）
  public async initializeChunks(filePath: string): Promise<void> {
    try {
      console.log("[CHUNK_MANAGER] 开始获取图片元数据...", filePath);

      // 保存当前文件路径
      this.currentFilePath = filePath;

      // 调用 Rust 获取元数据
      const { invoke } = await import("@tauri-apps/api/core");
      this.metadata = await invoke("get_image_metadata_for_file", {
        filePath: filePath,
      });

      console.log(
        `[CHUNK_MANAGER] 元数据获取成功: ${this.metadata.total_width}x${this.metadata.total_height}, 共 ${this.metadata.chunks.length} 个 chunks`
      );

      // 创建所有 chunk 对象
      this.metadata.chunks.forEach((chunkInfo) => {
        const chunk = new ImageChunk(chunkInfo);
        this.chunks.set(chunk.id, chunk);
      });

      console.log("[CHUNK_MANAGER] Chunk 初始化完成");
    } catch (error) {
      console.error("[CHUNK_MANAGER] 初始化失败:", error);
      throw error;
    }
  }

  // 请求 chunk 数据
  public async requestChunk(chunkId: string): Promise<void> {
    const chunk = this.chunks.get(chunkId);
    if (!chunk) {
      console.error(`[CHUNK_MANAGER] Chunk 不存在: ${chunkId}`);
      return;
    }

    if (
      chunk.status === ChunkStatus.REQUESTING ||
      chunk.status === ChunkStatus.IN_CPU ||
      chunk.status === ChunkStatus.IN_GPU
    ) {
      console.log(
        `[CHUNK_MANAGER] Chunk ${chunkId} 状态为 ${chunk.status}，跳过请求`
      );
      return;
    }

    // 添加到请求队列
    this.requestQueue.push(chunkId);
    this.processQueue();
  }

  // 处理请求队列
  private async processQueue(): Promise<void> {
    if (
      this.activeRequests >= this.maxConcurrentRequests ||
      this.requestQueue.length === 0
    ) {
      return;
    }

    const chunkId = this.requestQueue.shift();
    if (!chunkId) return;

    this.activeRequests++;
    const chunk = this.chunks.get(chunkId);
    if (!chunk) {
      this.activeRequests--;
      return;
    }

    try {
      chunk.updateStatus(ChunkStatus.REQUESTING);
      console.log(`[CHUNK_MANAGER] 开始请求 chunk: ${chunkId}`);

      // 调用 Rust 获取 chunk 数据（零拷贝版本）
      const { invoke } = await import("@tauri-apps/api/core");
      const rawData = await invoke("get_image_chunk", {
        chunkX: chunk.chunk_x,
        chunkY: chunk.chunk_y,
        filePath: this.currentFilePath,
      });

      console.log(`[CHUNK_MANAGER] 接收到的数据类型:`, typeof rawData);
      console.log(`[CHUNK_MANAGER] 接收到的数据结构:`, rawData);

      // 处理不同的数据类型：ArrayBuffer 或 Uint8Array
      let chunkData: Uint8Array;
      if (rawData instanceof ArrayBuffer) {
        console.log(`[CHUNK_MANAGER] 接收到 ArrayBuffer，转换为 Uint8Array`);
        chunkData = new Uint8Array(rawData);
      } else if (rawData instanceof Uint8Array) {
        console.log(`[CHUNK_MANAGER] 接收到 Uint8Array`);
        chunkData = rawData;
      } else {
        console.error(
          `[CHUNK_MANAGER] 数据类型错误: 期望 ArrayBuffer 或 Uint8Array，实际 ${typeof rawData}`
        );
        throw new Error(
          `数据类型错误: 期望 ArrayBuffer 或 Uint8Array，实际 ${typeof rawData}`
        );
      }

      // 解析二进制数据：宽度(4字节) + 高度(4字节) + 像素数据
      console.log(`[CHUNK_MANAGER] 接收到原始数据: ${chunkData.length} 字节`);

      if (chunkData.length < 8) {
        throw new Error(
          `Chunk 数据格式错误：数据长度不足 ${chunkData.length} 字节`
        );
      }

      // 解析宽度和高度（大端序）- 使用 DataView 正确处理32位无符号整数
      const dataView = new DataView(
        chunkData.buffer,
        chunkData.byteOffset,
        chunkData.byteLength
      );
      const width = dataView.getUint32(0, false); // false = 大端序
      const height = dataView.getUint32(4, false); // false = 大端序

      console.log(`[CHUNK_MANAGER] 解析的尺寸: ${width}x${height}`);

      // 提取像素数据
      const pixels = chunkData.slice(8);
      console.log(`[CHUNK_MANAGER] 提取的像素数据: ${pixels.length} 字节`);

      // 验证数据大小
      const expectedPixelsSize = width * height * 4; // RGBA = 4字节
      console.log(`[CHUNK_MANAGER] 期望像素大小: ${expectedPixelsSize} 字节`);

      if (pixels.length !== expectedPixelsSize) {
        throw new Error(
          `Chunk 数据大小不匹配：期望 ${expectedPixelsSize} 字节，实际 ${pixels.length} 字节`
        );
      }

      // 更新chunk的尺寸信息（以防与元数据不一致）
      chunk.width = width;
      chunk.height = height;

      chunk.setData(pixels);

      console.log(
        `[CHUNK_MANAGER] Chunk ${chunkId} 数据获取成功: ${pixels.length} 字节`
      );

      // 上传到 GPU
      await this.uploadChunkToGPU(chunk);
    } catch (error) {
      console.error(`[CHUNK_MANAGER] Chunk ${chunkId} 请求失败:`, error);
      chunk.updateStatus(ChunkStatus.ERROR);
    } finally {
      this.activeRequests--;
      // 继续处理队列
      this.processQueue();
    }
  }

  // 上传 chunk 到 GPU
  private async uploadChunkToGPU(chunk: ImageChunk): Promise<void> {
    if (!this.gl || !chunk.data) {
      console.error("[CHUNK_MANAGER] WebGL 上下文或 chunk 数据不可用");
      return;
    }

    try {
      // 创建纹理
      const texture = this.gl.createTexture();
      if (!texture) {
        throw new Error("无法创建 WebGL 纹理");
      }

      this.gl.bindTexture(this.gl.TEXTURE_2D, texture);
      this.gl.texParameteri(
        this.gl.TEXTURE_2D,
        this.gl.TEXTURE_WRAP_S,
        this.gl.CLAMP_TO_EDGE
      );
      this.gl.texParameteri(
        this.gl.TEXTURE_2D,
        this.gl.TEXTURE_WRAP_T,
        this.gl.CLAMP_TO_EDGE
      );
      this.gl.texParameteri(
        this.gl.TEXTURE_2D,
        this.gl.TEXTURE_MIN_FILTER,
        this.gl.NEAREST
      );
      this.gl.texParameteri(
        this.gl.TEXTURE_2D,
        this.gl.TEXTURE_MAG_FILTER,
        this.gl.NEAREST
      );

      // 上传像素数据
      this.gl.texImage2D(
        this.gl.TEXTURE_2D,
        0,
        this.gl.RGBA,
        chunk.width,
        chunk.height,
        0,
        this.gl.RGBA,
        this.gl.UNSIGNED_BYTE,
        chunk.data
      );

      chunk.setTexture(texture);
      console.log(`[CHUNK_MANAGER] Chunk ${chunk.id} 已上传到 GPU`);

      // 触发 chunk 就绪回调，立即渲染
      if (this.onChunkReady) {
        this.onChunkReady(chunk);
      }
    } catch (error) {
      console.error(`[CHUNK_MANAGER] Chunk ${chunk.id} GPU 上传失败:`, error);
      chunk.updateStatus(ChunkStatus.ERROR);
    }
  }

  // 获取所有已加载的 chunk
  public getLoadedChunks(): ImageChunk[] {
    return Array.from(this.chunks.values()).filter(
      (chunk) => chunk.status === ChunkStatus.IN_GPU
    );
  }

  // 获取所有 chunk IDs（用于测试）
  public getAllChunkIds(): string[] {
    return Array.from(this.chunks.keys());
  }

  // 获取元数据
  public getMetadata(): ImageMetadata | undefined {
    return this.metadata;
  }

  // 获取 chunk 状态统计
  public getStatusStats(): Record<ChunkStatus, number> {
    const stats: Record<ChunkStatus, number> = {
      [ChunkStatus.UNREQUESTED]: 0,
      [ChunkStatus.REQUESTING]: 0,
      [ChunkStatus.IN_CPU]: 0,
      [ChunkStatus.IN_GPU]: 0,
      [ChunkStatus.ERROR]: 0,
    };

    this.chunks.forEach((chunk) => {
      stats[chunk.status]++;
    });

    return stats;
  }

  // 清理资源
  public cleanup(): void {
    if (this.gl) {
      this.chunks.forEach((chunk) => {
        chunk.cleanup(this.gl!);
      });
    }
    this.chunks.clear();
    this.requestQueue = [];
    this.activeRequests = 0;
    console.log("[CHUNK_MANAGER] 资源清理完成");
  }
}
