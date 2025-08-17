import { ChunkManager, ChunkStatus } from "./chunk-manager";
import webglUtils from "../../utils/webgl";
import { getTime } from "../../utils/time";

console.log(`[CHUNK_TEST] 开始测试 chunk 机制: ${getTime()}`);

// 创建 chunk 管理器
const chunkManager = new ChunkManager();

// 初始化 WebGL
let canvas: HTMLCanvasElement | null = null;
let gl: WebGL2RenderingContext | null = null;
let program: WebGLProgram | null = null;
let vao: WebGLVertexArrayObject | null = null;
let positionBuffer: WebGLBuffer | null = null;
let texCoordBuffer: WebGLBuffer | null = null;

// 主函数
async function main() {
  try {
    // 初始化 WebGL
    await initializeWebGL();

    // 设置 WebGL 上下文到 chunk 管理器
    chunkManager.setWebGLContext(gl!);

    // 初始化 chunk 信息
    console.log("[CHUNK_TEST] 开始初始化 chunks...");
    await chunkManager.initializeChunks();

    // 开始加载所有 chunks
    console.log("[CHUNK_TEST] 开始加载 chunks...");
    await loadAllChunks();

    console.log("[CHUNK_TEST] 所有 chunks 加载完成");
  } catch (error) {
    console.error("[CHUNK_TEST] 初始化失败:", error);
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
    new Float32Array([
      0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0,
    ]),
    gl.STATIC_DRAW
  );
  gl.enableVertexAttribArray(texCoordAttributeLocation);
  gl.vertexAttribPointer(texCoordAttributeLocation, 2, gl.FLOAT, false, 0, 0);

  console.log("[CHUNK_TEST] WebGL 初始化完成");
}

// 加载所有 chunks
async function loadAllChunks(): Promise<void> {
  const startTime = getTime();
  console.log("[CHUNK_TEST] 开始加载所有 chunks...");

  // 获取所有 chunk IDs
  const chunkIds = chunkManager.getAllChunkIds();
  console.log(`[CHUNK_TEST] 共有 ${chunkIds.length} 个 chunks 需要加载`);

  // 逐个请求 chunks
  for (const chunkId of chunkIds) {
    await chunkManager.requestChunk(chunkId);

    // 等待一小段时间，避免同时发起太多请求
    await new Promise((resolve) => setTimeout(resolve, 100));

    // 显示进度
    const stats = chunkManager.getStatusStats();
    console.log(
      `[CHUNK_TEST] 进度: ${stats[ChunkStatus.IN_GPU]}/${
        chunkIds.length
      } chunks 已加载到 GPU`
    );
  }

  const endTime = getTime();
  console.log(
    `[CHUNK_TEST] 所有 chunks 加载完成: ${endTime}ms (总耗时: ${
      endTime - startTime
    }ms)`
  );

  // 开始渲染
  startRendering();
}

// 开始渲染
function startRendering(): void {
  console.log("[CHUNK_TEST] 开始渲染...");

  // 设置定时器，定期渲染
  setInterval(() => {
    renderChunks();
  }, 100); // 每 100ms 渲染一次
}

// 渲染 chunks
function renderChunks(): void {
  if (!gl || !program || !canvas) return;

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

  // 获取所有已加载的 chunks
  const loadedChunks = chunkManager.getLoadedChunks();

  // 渲染每个 chunk
  loadedChunks.forEach((chunk, index) => {
    if (chunk.texture) {
      // 绑定纹理
      gl!.activeTexture(gl!.TEXTURE0);
      gl!.bindTexture(gl!.TEXTURE_2D, chunk.texture);

      // 设置矩形位置（简单的平铺显示）
      const chunkSize = 200; // 显示大小
      const x = (index % 4) * chunkSize;
      const y = Math.floor(index / 4) * chunkSize;

      // 设置矩形位置
      setRectangle(gl!, x, y, chunkSize, chunkSize);

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

// 等待 DOM 加载完成后再启动
document.addEventListener("DOMContentLoaded", () => {
  // 启动主函数
  main().catch(console.error);
});

// 导出 chunk 管理器，方便调试
(window as any).chunkManager = chunkManager;
