import { invoke } from "@tauri-apps/api/core";
import webglUtils from "../../utils/webgl";
import { getTime } from "../../utils/time";

console.log(`[TS] 开始加载worker: ${getTime()}`);
// 创建 worker 并赋值给 window
const worker = new Worker(
  new URL("./worker/image-worker.ts", import.meta.url),
  {
    type: "module",
  }
);
// 使用 Web Worker 处理图片数据

async function getImage() {
  const startTime = getTime();

  try {
    const invokeStart = getTime();
    console.log(`[TS] 开始调用Rust: read_file ${invokeStart}ms`);
    const imageBuffer = await invoke("read_file");
    const invokeEnd = getTime();
    console.log(
      `[TS] Rust调用完成: ${invokeEnd}ms (耗时: ${invokeEnd - invokeStart}ms)`
    );

    if (imageBuffer instanceof ArrayBuffer) {
      console.log("成功读取图片:", imageBuffer);
      console.log(`[TS] 开始Worker处理: ${getTime()}ms`);

      worker.onmessage = function (e) {
        const workerEndTime = getTime();
        const { success, imageBitmap, width, height, error } = e.data;

        if (success) {
          console.log(`[TS] Worker处理完成: ${workerEndTime}ms`);
          console.log("Worker 处理完成，图片尺寸:", width, "x", height);

          const webglStartTime = getTime();

          // 初始化 WebGL 上下文和纹理
          canvas = document.querySelector("#canvas") as HTMLCanvasElement;
          gl = canvas.getContext("webgl2");

          if (!gl) {
            console.error("WebGL2 不可用");
            return;
          }

          // 创建纹理
          texture = gl.createTexture();
          gl.bindTexture(gl.TEXTURE_2D, texture);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

          // 直接使用 ImageBitmap 上传纹理（最佳性能）
          gl.texImage2D(
            gl.TEXTURE_2D,
            0,
            gl.RGBA,
            gl.RGBA,
            gl.UNSIGNED_BYTE,
            imageBitmap
          );

          const textureEndTime = getTime();
          console.log(
            `[TS] 纹理上传完成: ${textureEndTime}ms (耗时: ${
              textureEndTime - webglStartTime
            }ms)`
          );

          currentImage = imageBitmap; // 直接使用 Worker 返回的 ImageBitmap
          console.log(
            `[TS] 接收到 ImageBitmap: ${imageBitmap.width}x${imageBitmap.height}, 类型: ${imageBitmap.constructor.name}`
          );

          // 初始化其他 WebGL 资源并渲染
          const initStartTime = getTime();
          initializeWebGLResources(imageBitmap);
          const initEndTime = getTime();
          console.log(
            `[TS] WebGL初始化完成: ${initEndTime}ms (耗时: ${
              initEndTime - initStartTime
            }ms)`
          );

          // 清理 Worker
          worker.terminate();

          const totalEndTime = getTime();
          console.log(
            `[TS] 图片加载完成: ${totalEndTime}ms (总耗时: ${
              totalEndTime - startTime
            }ms)`
          );
        } else {
          console.error("Worker 处理失败:", error);
        }
      };

      // 零拷贝传输：转移 ArrayBuffer 所有权给 Worker
      const workerStartTime = getTime();
      worker.postMessage(
        {
          imageBuffer: imageBuffer,
        },
        [imageBuffer]
      ); // 关键：转移所有权，避免复制
      const workerPostTime = getTime();
      console.log(
        `[TS] 发送到Worker完成: ${workerPostTime}ms (耗时: ${
          workerPostTime - workerStartTime
        }ms)`
      );

      console.log("已发送图片数据到 Worker 处理");
    } else {
      console.error("意外的数据类型:", typeof imageBuffer, imageBuffer);
    }
  } catch (error) {
    console.error("读取图片失败:", error);
  }
}

// 初始化滚轮事件（当 DOM 加载完成后）
document.addEventListener("DOMContentLoaded", () => {
  setupWheelEvent();
});

const vertexShaderSource = /*glsl*/ `#version 300 es
  #pragma vscode_glsllint_stage : vert //pragma to set STAGE to 'frag'
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

const fragmentShaderSource = /*glsl*/ `#version 300 es
  #pragma vscode_glsllint_stage : frag
  precision highp float;
  uniform sampler2D u_image;
  in vec2 v_texCoord;
  out vec4 outColor;
  void main() {
    outColor = texture(u_image, v_texCoord);
  }
`;

// 视图状态管理
interface ViewState {
  scale: number;
  offsetX: number;
  offsetY: number;
}

let viewState: ViewState = {
  scale: 1.0,
  offsetX: 0,
  offsetY: 0,
};

let canvas: HTMLCanvasElement | null = null;
let gl: WebGL2RenderingContext | null = null;
let program: WebGLProgram | null = null;
let vao: WebGLVertexArrayObject | null = null;
let positionBuffer: WebGLBuffer | null = null;
let texCoordBuffer: WebGLBuffer | null = null;
let texture: WebGLTexture | null = null;
let currentImage: HTMLImageElement | ImageBitmap | null = null;
let wheelEventSetup = false;

// 初始化 WebGL 资源（除了纹理，纹理已经在 getImage 中创建）
function initializeWebGLResources(image: HTMLImageElement | ImageBitmap): void {
  if (!canvas || !gl || !texture) {
    console.error("WebGL 上下文或纹理未初始化");
    return;
  }

  currentImage = image;

  // 计算初始缩放比例，使图片完全显示在 canvas 中
  calculateInitialScale();

  // setup GLSL program
  program = webglUtils.createProgramFromSources(gl, [
    vertexShaderSource,
    fragmentShaderSource,
  ]);

  if (!program) {
    console.error("Failed to create WebGL program");
    return;
  }

  // look up where the vertex data needs to go.
  const positionAttributeLocation = gl.getAttribLocation(program, "a_position");
  const texCoordAttributeLocation = gl.getAttribLocation(program, "a_texCoord");

  // Create a vertex array object (attribute state)
  vao = gl.createVertexArray();
  if (!vao) {
    console.error("Failed to create vertex array object");
    return;
  }

  // and make it the one we're currently working with
  gl.bindVertexArray(vao);

  // Create a buffer and put a single pixel space rectangle in
  // it (2 triangles)
  positionBuffer = gl.createBuffer();
  if (!positionBuffer) {
    console.error("Failed to create position buffer");
    return;
  }

  // Turn on the attribute
  gl.enableVertexAttribArray(positionAttributeLocation);

  // Bind it to ARRAY_BUFFER (think of it as ARRAY_BUFFER = positionBuffer)
  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);

  // Tell the attribute how to get data out of positionBuffer (ARRAY_BUFFER)
  const size = 2; // 2 components per iteration
  const type = gl.FLOAT; // the data is 32bit floats
  const normalize = false; // don't normalize the data
  const stride = 0; // 0 = move forward size * sizeof(type) each iteration to get the next position
  const offset = 0; // start at the beginning of the buffer
  gl.vertexAttribPointer(
    positionAttributeLocation,
    size,
    type,
    normalize,
    stride,
    offset
  );

  // provide texture coordinates for the rectangle.
  texCoordBuffer = gl.createBuffer();
  if (!texCoordBuffer) {
    console.error("Failed to create texture coordinate buffer");
    return;
  }

  gl.bindBuffer(gl.ARRAY_BUFFER, texCoordBuffer);
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([
      0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0,
    ]),
    gl.STATIC_DRAW
  );

  // Turn on the attribute
  gl.enableVertexAttribArray(texCoordAttributeLocation);

  // Tell the attribute how to get data out of texCoordBuffer (ARRAY_BUFFER)
  gl.vertexAttribPointer(
    texCoordAttributeLocation,
    size,
    type,
    normalize,
    stride,
    0
  );

  // 设置滚轮事件（只在第一次渲染时设置）
  if (!wheelEventSetup) {
    setupWheelEvent();
    wheelEventSetup = true;
  }

  // 初始绘制
  redraw();
}

// 计算初始缩放比例，使图片完全显示
function calculateInitialScale(): void {
  if (!canvas || !currentImage) return;

  const canvasWidth = canvas.clientWidth;
  const canvasHeight = canvas.clientHeight;
  const imageWidth = currentImage.width;
  const imageHeight = currentImage.height;

  // 计算缩放比例，使图片完全适应 canvas
  const scaleX = canvasWidth / imageWidth;
  const scaleY = canvasHeight / imageHeight;
  const scale = Math.min(scaleX, scaleY, 1.0); // 不超过 100%

  viewState.scale = scale;
  viewState.offsetX = (canvasWidth - imageWidth * scale) / 2;
  viewState.offsetY = (canvasHeight - imageHeight * scale) / 2;

  console.log(
    `初始缩放: ${scale.toFixed(2)}, 偏移: (${viewState.offsetX.toFixed(
      0
    )}, ${viewState.offsetY.toFixed(0)})`
  );
}

// 重新绘制图片
function redraw(): void {
  if (!gl || !program || !currentImage || !canvas) return;

  // 确保 canvas 尺寸正确
  webglUtils.resizeCanvasToDisplaySize(canvas);

  // 设置视口
  gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);

  // 清除 canvas
  gl.clearColor(0, 0, 0, 1); // 设置为黑色背景
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

  // 绑定纹理
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, texture);

  // 计算缩放后的矩形位置
  const scaledWidth = currentImage.width * viewState.scale;
  const scaledHeight = currentImage.height * viewState.scale;

  // 绑定位置缓冲区
  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);

  // 设置矩形位置（考虑偏移）
  setRectangle(
    gl,
    viewState.offsetX,
    viewState.offsetY,
    scaledWidth,
    scaledHeight
  );

  // 绘制
  gl.drawArrays(gl.TRIANGLES, 0, 6);
}

// 设置滚轮缩放事件
function setupWheelEvent(): void {
  if (!canvas) return;

  const canvasElement = canvas; // 创建一个局部引用

  canvasElement.addEventListener("wheel", (event) => {
    event.preventDefault();

    const delta = event.deltaY > 0 ? 0.9 : 1.1; // 滚轮向下缩小，向上放大
    const newScale = viewState.scale * delta;

    // 限制缩放范围
    if (newScale >= 0.1 && newScale <= 5.0) {
      viewState.scale = newScale;

      // 计算鼠标位置相对于图片的偏移
      const rect = canvasElement.getBoundingClientRect();
      const mouseX = event.clientX - rect.left;
      const mouseY = event.clientY - rect.top;

      // 调整偏移以保持鼠标位置不变
      const scaleDiff = delta - 1;
      viewState.offsetX -= (mouseX - viewState.offsetX) * scaleDiff;
      viewState.offsetY -= (mouseY - viewState.offsetY) * scaleDiff;

      // 重新绘制
      redraw();
    }
  });
}

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
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([x1, y1, x2, y1, x1, y2, x1, y2, x2, y1, x2, y2]),
    gl.STATIC_DRAW
  );
}
