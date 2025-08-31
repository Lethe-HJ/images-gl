<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { ChunkManager, ChunkStatus, ImageMetadata } from "@/render/image/chunk-manager";
import webglUtils from "@/utils/webgl";
import { getTime } from "@/utils/time";
import { open } from '@tauri-apps/plugin-dialog';

// 创建 chunk 管理器
const chunkManager = new ChunkManager();

// 自定义文件类型
interface SelectedFile {
  name: string;
  path: string;
  size: number;
}

// 状态管理
const isInitialized = ref(false);
const isProcessing = ref(false);
const selectedFile = ref<SelectedFile | null>(null);
const statusMessage = ref('请选择图片文件');
const statusColor = ref('#FFC107');

// DOM引用
const fileInputRef = ref<HTMLInputElement | null>(null);

// 触发文件选择
function triggerFileSelect() {
  handleFileSelect();
}

// 初始化 WebGL
let canvas: HTMLCanvasElement | null = null;
let gl: WebGL2RenderingContext | null = null;
let program: WebGLProgram | null = null;
let vao: WebGLVertexArrayObject | null = null;
let positionBuffer: WebGLBuffer | null = null;
let texCoordBuffer: WebGLBuffer | null = null;

// 文件选择处理
async function handleFileSelect() {
  try {
    // 根据Tauri v2文档，使用正确的导入方式
    const selectedPath = await open({
      title: '选择要处理的图片文件',
      directory: false,
      multiple: false,
      filters: [
        {
          name: '图片文件',
          extensions: ['png', 'jpg', 'jpeg', 'bmp', 'tiff', 'webp']
        }
      ]
    });

    if (!selectedPath || Array.isArray(selectedPath)) {
      statusMessage.value = '未选择文件';
      statusColor.value = '#FF6B6B';
      return;
    }

    // 设置选中的文件信息
    selectedFile.value = {
      name: selectedPath.split('/').pop() || selectedPath.split('\\').pop() || '未知文件',
      path: selectedPath,
      size: 0 // 我们无法直接获取文件大小，但这不是必需的
    };

    statusMessage.value = `已选择: ${selectedFile.value.name}`;
    statusColor.value = '#4CAF50';

    processSelectedImage();

  } catch (error) {
    console.error('[IMAGE_VIEWER] 文件选择失败:', error);
    statusMessage.value = '文件选择失败，请检查Tauri配置';
    statusColor.value = '#FF6B6B';
  }
}

// 处理选择的图片
async function processSelectedImage() {
  if (!selectedFile.value) {
    statusMessage.value = '请先选择图片文件';
    statusColor.value = '#FF6B6B';
    return;
  }

  try {
    isProcessing.value = true;
    statusMessage.value = '正在处理图片...';
    statusColor.value = '#FFC107';

    // 初始化 WebGL（如果还没有初始化）
    if (!isInitialized.value) {
      await initializeWebGL();
      chunkManager.setWebGLContext(gl!);
      chunkManager.setOnChunkReady((chunk) => {
        console.log(`[IMAGE_VIEWER] Chunk ${chunk.id} 就绪，立即渲染`);
        renderChunks();
      });
      isInitialized.value = true;
    }

    // 调用后端处理图片
    const { invoke } = await import('@tauri-apps/api/core');
    const metadata = await invoke('process_user_image', { filePath: selectedFile.value.path }) as ImageMetadata;
    console.log('[IMAGE_VIEWER] 图片处理完成:', metadata);

    // 直接使用返回的元数据初始化 chunks，避免重复调用后端
    await chunkManager.initializeChunksFromMetadata(selectedFile.value.path, metadata);

    // 开始加载 chunks
    await loadAllChunks();

    statusMessage.value = '图片处理完成，开始加载chunks...';
    statusColor.value = '#4CAF50';

  } catch (error) {
    console.error('[IMAGE_VIEWER] 处理图片失败:', error);
    statusMessage.value = `处理失败: ${error instanceof Error ? error.message : String(error)}`;
    statusColor.value = '#FF6B6B';
  } finally {
    isProcessing.value = false;
  }
}

// 主函数（现在由用户手动触发）
async function main() {
  try {
    // 初始化 WebGL
    await initializeWebGL();

    // 设置 WebGL 上下文到 chunk 管理器
    chunkManager.setWebGLContext(gl!);

    // 设置 chunk 就绪回调，实现逐步渲染
    chunkManager.setOnChunkReady((chunk) => {
      console.log(`[IMAGE_VIEWER] Chunk ${chunk.id} 就绪，立即渲染`);
      // 立即渲染当前可用的所有 chunks
      renderChunks();
    });

    isInitialized.value = true;
    statusMessage.value = 'WebGL初始化完成，请选择图片文件';
    statusColor.value = '#4CAF50';

  } catch (error) {
    console.error('[IMAGE_VIEWER] 初始化失败:', error);
    statusMessage.value = `WebGL初始化失败: ${error instanceof Error ? error.message : String(error)}`;
    statusColor.value = '#FF6B6B';
  }
}

// 初始化 WebGL
async function initializeWebGL(): Promise<void> {
  canvas = document.querySelector("#canvas") as HTMLCanvasElement;
  if (!canvas) {
    throw new Error("Canvas 元素未找到");
  }

  gl = canvas.getContext("webgl2");
  if (!gl) {
    throw new Error("WebGL2 不可用");
  }

  // 设置 canvas 尺寸
  webglUtils.resizeCanvasToDisplaySize(canvas);

  // 创建着色器程序
  const vertexShaderSource = `#version 300 es
    in vec2 a_position;
    in vec2 a_texCoord;
    uniform vec2 u_resolution;
    out vec2 v_texCoord;
    void main() {
      vec2 zeroToOne = a_position / u_resolution;
      vec2 zeroToTwo = zeroToOne * 2.0;
      vec2 clipSpace = zeroToTwo - 1.0;
      gl_Position = vec4(clipSpace * vec2(1, -1), 0, 1);
      v_texCoord = a_texCoord;
    }
  `;

  const fragmentShaderSource = `#version 300 es
    precision highp float;
    uniform sampler2D u_image;
    in vec2 v_texCoord;
    out vec4 outColor;
    void main() {
      outColor = texture(u_image, v_texCoord);
    }
  `;

  program = webglUtils.createProgramFromSources(gl, [
    vertexShaderSource,
    fragmentShaderSource,
  ]);

  if (!program) {
    throw new Error("Failed to create WebGL program");
  }

  // 设置顶点属性
  const positionAttributeLocation = gl.getAttribLocation(program, "a_position");
  const texCoordAttributeLocation = gl.getAttribLocation(program, "a_texCoord");

  vao = gl.createVertexArray();
  gl.bindVertexArray(vao);

  // 位置缓冲区
  positionBuffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  gl.enableVertexAttribArray(positionAttributeLocation);
  gl.vertexAttribPointer(positionAttributeLocation, 2, gl.FLOAT, false, 0, 0);

  // 纹理坐标缓冲区
  texCoordBuffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, texCoordBuffer);
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0]),
    gl.STATIC_DRAW
  );
  gl.enableVertexAttribArray(texCoordAttributeLocation);
  gl.vertexAttribPointer(texCoordAttributeLocation, 2, gl.FLOAT, false, 0, 0);

  console.log('[IMAGE_VIEWER] WebGL 初始化完成');
}

// 百叶窗式加载配置
const CHUNK_LOAD_INTERVAL = 500; // 每个批次之间的间隔时间（毫秒）

// 加载所有 chunks（空间间隔的百叶窗式）
async function loadAllChunks(): Promise<void> {
  const startTime = getTime();
  // 获取所有 chunk IDs
  const chunkIds = chunkManager.getAllChunkIds();
  console.log(`[IMAGE_VIEWER] 共有 ${chunkIds.length} 个 chunks 需要加载`);

  // 获取元数据以了解 chunk 的网格布局
  const metadata = chunkManager.getMetadata();
  if (!metadata) {
    console.error('[IMAGE_VIEWER] 无法获取元数据');
    return;
  }

  // 计算网格尺寸 - 使用元数据中的实际 chunk_size
  const gridWidth = Math.ceil(metadata.total_width / metadata.chunk_size);
  const gridHeight = Math.ceil(metadata.total_height / metadata.chunk_size);
  console.log(`[IMAGE_VIEWER] Chunk 网格: ${gridWidth}x${gridHeight}`);

  // 创建空间间隔的批次
  const batches = createSpatialBatches(gridWidth, gridHeight);
  console.log(`[IMAGE_VIEWER] 创建了 ${batches.length} 个空间间隔批次`);

  // 按批次加载
  for (let batchIndex = 0; batchIndex < batches.length; batchIndex++) {
    const batch = batches[batchIndex];
    console.log(`[IMAGE_VIEWER] 加载批次 ${batchIndex + 1}: ${batch.map(id => id).join(', ')}`);

    // 同时请求这一批的 chunks
    const promises = batch.map(chunkId => {
      console.log(`[IMAGE_VIEWER] 开始加载 chunk: ${chunkId}`);
      return chunkManager.requestChunk(chunkId);
    });

    // 等待这一批完成
    await Promise.all(promises);
    console.log(`[IMAGE_VIEWER] 批次 ${batchIndex + 1} 完成`);

    // 显示进度
    const stats = chunkManager.getStatusStats();
    console.log(`[IMAGE_VIEWER] 进度: ${stats[ChunkStatus.IN_GPU]}/${chunkIds.length} chunks 已加载到 GPU`);
  }

  const endTime = getTime();
  console.log(`[IMAGE_VIEWER] 所有 chunks 加载完成: ${endTime}ms (总耗时: ${endTime - startTime}ms)`);

  // 开始渲染
  startRendering();
}

// 创建空间间隔的批次，确保同一批次中的 chunks 不相邻
function createSpatialBatches(gridWidth: number, gridHeight: number): string[][] {
  const batches: string[][] = [];
  const visited = new Set<string>();

  // 第一批：奇数行奇数列 (1,1), (1,3), (3,1), (3,3)...
  const batch1: string[] = [];
  for (let y = 1; y < gridHeight; y += 2) {
    for (let x = 1; x < gridWidth; x += 2) {
      const chunkId = `${x}_${y}`;
      batch1.push(chunkId);
      visited.add(chunkId);
    }
  }
  if (batch1.length > 0) batches.push(batch1);

  // 第二批：偶数行偶数列 (0,0), (0,2), (2,0), (2,2)...
  const batch2: string[] = [];
  for (let y = 0; y < gridHeight; y += 2) {
    for (let x = 0; x < gridWidth; x += 2) {
      const chunkId = `${x}_${y}`;
      batch2.push(chunkId);
      visited.add(chunkId);
    }
  }
  if (batch2.length > 0) batches.push(batch2);

  // 第三批：奇数行偶数列 (1,0), (1,2), (3,0), (3,2)...
  const batch3: string[] = [];
  for (let y = 0; y < gridHeight; y += 2) {
    for (let x = 1; x < gridWidth; x += 2) {
      const chunkId = `${x}_${y}`;
      batch3.push(chunkId);
      visited.add(chunkId);
    }
  }
  if (batch3.length > 0) batches.push(batch3);

  // 第四批：偶数行奇数列 (0,1), (0,3), (2,1), (2,3)...
  const batch4: string[] = [];
  for (let y = 1; y < gridHeight; y += 2) {
    for (let x = 0; x < gridWidth; x += 2) {
      const chunkId = `${x}_${y}`;
      batch4.push(chunkId);
      visited.add(chunkId);
    }
  }
  if (batch4.length > 0) batches.push(batch4);

  console.log(`[IMAGE_VIEWER] 空间批次分布:`);
  batches.forEach((batch, index) => {
    console.log(`  批次 ${index + 1}: ${batch.join(', ')}`);
  });

  return batches;
}

// 开始渲染（现在由回调触发，不需要定时器）
function startRendering(): void {
  console.log('[IMAGE_VIEWER] 渲染系统已启动，由 chunk 就绪回调触发');
}

// 渲染 chunks
function renderChunks(): void {
  if (!gl || !program || !canvas) return;

  // 更新状态显示和进度条
  const stats = chunkManager.getStatusStats();
  const totalChunks = chunkManager.getAllChunkIds().length;
  const loadedChunksCount = stats[ChunkStatus.IN_GPU];

  const statusElement = document.getElementById('status');
  if (statusElement) {
    statusElement.textContent = `已加载: ${loadedChunksCount}/${totalChunks} chunks`;
  }

  // 更新进度条
  const progressElement = document.querySelector('.progress-fill') as HTMLElement;
  if (progressElement && totalChunks > 0) {
    const progress = (loadedChunksCount / totalChunks) * 100;
    progressElement.style.width = `${progress}%`;
  }

  // 确保 canvas 尺寸正确
  webglUtils.resizeCanvasToDisplaySize(canvas);

  // 设置视口
  gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);

  // 清除 canvas
  gl.clearColor(0, 0, 0, 1);
  gl.clear(gl.COLOR_BUFFER_BIT);

  // 使用程序
  gl.useProgram(program);

  // 绑定 VAO
  if (vao) gl.bindVertexArray(vao);

  // 设置 uniforms
  const resolutionLocation = gl.getUniformLocation(program, "u_resolution");
  const imageLocation = gl.getUniformLocation(program, "u_image");

  gl.uniform2f(resolutionLocation, gl.canvas.width, gl.canvas.height);
  gl.uniform1i(imageLocation, 0);

  // 固定视口：使用元数据中的总尺寸，不随 chunks 加载而变化
  if (!chunkManager.getMetadata()) return;

  const metadata = chunkManager.getMetadata()!;
  const imageWidth = metadata.total_width;
  const imageHeight = metadata.total_height;
  const canvasWidth = gl.canvas.width;
  const canvasHeight = gl.canvas.height;

  // 计算缩放比例，使图片完全适应 canvas
  const scaleX = canvasWidth / imageWidth;
  const scaleY = canvasHeight / imageHeight;
  const scale = Math.min(scaleX, scaleY, 1.0); // 不超过 100%

  // 计算居中偏移
  const offsetX = (canvasWidth - imageWidth * scale) / 2;
  const offsetY = (canvasHeight - imageHeight * scale) / 2;

  // 获取已加载的 chunks
  const loadedChunks = chunkManager.getLoadedChunks();
  if (loadedChunks.length === 0) return;

  // 只在第一次渲染时打印视口信息
  if (loadedChunks.length === 1) {
    console.log(`[IMAGE_VIEWER] 固定视口: 图片尺寸(${imageWidth}x${imageHeight}), 缩放=${scale.toFixed(2)}, 偏移=(${offsetX.toFixed(0)}, ${offsetY.toFixed(0)})`);
  }

  // 渲染每个 chunk
  loadedChunks.forEach((chunk, index) => {
    if (chunk.texture) {
      // 绑定纹理
      gl!.activeTexture(gl!.TEXTURE0);
      gl!.bindTexture(gl!.TEXTURE_2D, chunk.texture);

      // 应用视口变换：将 chunk 坐标转换为屏幕坐标
      const screenX = chunk.x * scale + offsetX;
      const screenY = chunk.y * scale + offsetY;
      const screenWidth = chunk.width * scale;
      const screenHeight = chunk.height * scale;

      // 为新加载的 chunk 添加高亮效果（最后几个 chunk 会有不同的颜色）
      const isRecent = index >= loadedChunks.length - 3;
      if (isRecent) {
        // 为新加载的 chunk 添加边框效果
        console.log(`[IMAGE_VIEWER] 渲染 chunk ${chunk.id} (新加载): 原始位置(${chunk.x}, ${chunk.y}), 屏幕位置(${screenX.toFixed(0)}, ${screenY.toFixed(0)})`);
      } else {
        console.log(`[IMAGE_VIEWER] 渲染 chunk ${chunk.id}: 原始位置(${chunk.x}, ${chunk.y}), 屏幕位置(${screenX.toFixed(0)}, ${screenY.toFixed(0)})`);
      }

      // 设置矩形位置
      setRectangle(gl!, screenX, screenY, screenWidth, screenHeight);

      // 绘制
      gl!.drawArrays(gl!.TRIANGLES, 0, 6);
    }
  });
}

// 设置矩形位置
function setRectangle(
  gl: WebGL2RenderingContext,
  x: number,
  y: number,
  width: number,
  height: number
): void {
  const x1 = x;
  const x2 = x + width;
  const y1 = y;
  const y2 = y + height;
  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([x1, y1, x2, y1, x1, y2, x1, y2, x2, y1, x2, y2]),
    gl.STATIC_DRAW
  );
}

// 强制预处理函数
async function forcePreprocess() {
  try {
    console.log('[IMAGE_VIEWER] 手动触发强制预处理...');
    const { invoke } = await import('@tauri-apps/api/core');

    if (!selectedFile.value?.path) {
      console.error('[IMAGE_VIEWER] 没有选择文件，无法强制预处理');
      return;
    }

    const metadata = await invoke('force_preprocess_chunks', { filePath: selectedFile.value.path }) as ImageMetadata;
    console.log('[IMAGE_VIEWER] 强制预处理完成，重新初始化...');

    // 使用返回的元数据重新初始化
    await chunkManager.initializeChunksFromMetadata(selectedFile.value.path, metadata);
    await loadAllChunks();
  } catch (error) {
    console.error('[IMAGE_VIEWER] 强制预处理失败:', error);
  }
}

// 清理缓存函数
async function clearCache() {
  try {
    console.log('[IMAGE_VIEWER] 清理缓存...');
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('clear_chunk_cache');
    console.log('[IMAGE_VIEWER] 缓存已清理');
  } catch (error) {
    console.error('[IMAGE_VIEWER] 清理缓存失败:', error);
  }
}

// 重试初始化函数
async function retryInitialization() {
  try {
    console.log('[IMAGE_VIEWER] 重试初始化...');

    // 重置状态
    const statusElement = document.getElementById('status');
    if (statusElement) {
      statusElement.textContent = '重试初始化中...';
      statusElement.style.color = '#FFC107';
    }

    // 隐藏重试按钮
    const retryBtn = document.getElementById('retry-btn');
    if (retryBtn) {
      retryBtn.style.display = 'none';
    }

    // 重新初始化
    await main();
  } catch (error) {
    console.error('[IMAGE_VIEWER] 重试失败:', error);
  }
}

// 在组件挂载后初始化
onMounted(() => {
  // 延迟一小段时间，确保组件完全显示
  setTimeout(() => {
    // 启动主函数
    main().catch(console.error);

    // 导出 chunk 管理器，方便调试
    (window as any).chunkManager = chunkManager;
  }, 100);
});
</script>

<template>
  <div class="image-viewer">
    <div class="info-panel">
      <h2>图片分块加载</h2>

      <!-- 文件选择区域 -->
      <div class="file-selection">
        <input type="file" id="file-input" accept="image/*" @change="handleFileSelect" style="display: none;"
          ref="fileInputRef" />
        <button @click="triggerFileSelect" :disabled="isProcessing" class="file-select-btn">
          选择图片文件
        </button>
      </div>

      <!-- 状态显示 -->
      <div id="status" :style="{ color: statusColor }">{{ statusMessage }}</div>

      <!-- 进度条 -->
      <div id="progress" class="progress-bar" v-if="isInitialized">
        <div class="progress-fill"></div>
      </div>

      <!-- 控制按钮 -->
      <div class="controls" v-if="isInitialized">
        <button @click="forcePreprocess">强制预处理</button>
        <button @click="clearCache">清理缓存</button>
        <button @click="retryInitialization" id="retry-btn" style="display: none;">重试</button>
      </div>
    </div>
    <canvas id="canvas"></canvas>
  </div>
</template>

<style scoped>
.image-viewer {
  position: relative;
  width: 100vw;
  height: 100vh;
}

.info-panel {
  position: absolute;
  top: 20px;
  left: 20px;
  background: rgba(0, 0, 0, 0.8);
  color: white;
  padding: 20px;
  border-radius: 8px;
  z-index: 1000;
  font-family: monospace;
  font-size: 14px;
}

.info-panel h2 {
  margin: 0 0 10px 0;
  color: #4CAF50;
}

#status {
  color: #FFC107;
  margin-bottom: 10px;
}

.progress-bar {
  width: 100%;
  height: 8px;
  background: rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 15px;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #4CAF50, #8BC34A);
  width: 0%;
  transition: width 0.3s ease;
  border-radius: 4px;
}

.controls {
  margin-top: 15px;
  display: flex;
  gap: 10px;
}

.controls button {
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  background: #2196F3;
  color: white;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.3s ease;
}

.controls button:hover {
  background: #1976D2;
}

.controls button:last-child {
  background: #F44336;
}

.controls button:last-child:hover {
  background: #D32F2F;
}

.file-selection {
  margin-bottom: 15px;
  padding: 15px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 6px;
}

.file-select-btn {
  padding: 10px 20px;
  border: none;
  border-radius: 6px;
  background: #2196F3;
  color: white;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.3s ease;
  margin-bottom: 10px;
}

.file-select-btn:hover:not(:disabled) {
  background: #1976D2;
}

.file-select-btn:disabled {
  background: #9E9E9E;
  cursor: not-allowed;
}

.selected-file {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 10px;
}

.selected-file span {
  color: #4CAF50;
  font-weight: 500;
}

.process-btn {
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  background: #4CAF50;
  color: white;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.3s ease;
}

.process-btn:hover:not(:disabled) {
  background: #45A049;
}

.process-btn:disabled {
  background: #9E9E9E;
  cursor: not-allowed;
}

canvas {
  width: 100vw;
  height: 100vh;
  display: block;
}
</style>
