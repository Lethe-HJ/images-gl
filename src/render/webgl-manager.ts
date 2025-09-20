import webglUtils from '../../utils/webgl';

// WebGL 着色器源码
export const VERTEX_SHADER_SOURCE = `#version 300 es
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

export const FRAGMENT_SHADER_SOURCE = `#version 300 es
  precision highp float;
  uniform sampler2D u_image;
  in vec2 v_texCoord;
  out vec4 outColor;
  void main() {
    outColor = texture(u_image, v_texCoord);
  }
`;

// WebGL 上下文和资源接口
export interface WebGLResources {
  canvas: HTMLCanvasElement;
  gl: WebGL2RenderingContext;
  program: WebGLProgram;
  vao: WebGLVertexArrayObject;
  positionBuffer: WebGLBuffer;
  texCoordBuffer: WebGLBuffer;
}

// WebGL 管理器类
export class WebGLManager {
  private resources?: WebGLResources;

  // 初始化 WebGL 上下文和资源
  public async initialize(canvasId: string): Promise<WebGLResources> {
    const canvas = document.querySelector(canvasId) as HTMLCanvasElement;
    if (!canvas) {
      throw new Error(`Canvas 元素未找到: ${canvasId}`);
    }

    const gl = canvas.getContext('webgl2');
    if (!gl) {
      throw new Error('WebGL2 不可用');
    }

    // 设置 canvas 尺寸
    webglUtils.resizeCanvasToDisplaySize(canvas);

    // 创建着色器程序
    const program = webglUtils.createProgramFromSources(gl, [
      VERTEX_SHADER_SOURCE,
      FRAGMENT_SHADER_SOURCE,
    ]);

    if (!program) {
      throw new Error('Failed to create WebGL program');
    }

    // 设置顶点属性
    const positionAttributeLocation = gl.getAttribLocation(
      program,
      'a_position'
    );
    const texCoordAttributeLocation = gl.getAttribLocation(
      program,
      'a_texCoord'
    );

    const vao = gl.createVertexArray();
    if (!vao) {
      throw new Error('Failed to create vertex array object');
    }
    gl.bindVertexArray(vao);

    // 位置缓冲区
    const positionBuffer = gl.createBuffer();
    if (!positionBuffer) {
      throw new Error('Failed to create position buffer');
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
    gl.enableVertexAttribArray(positionAttributeLocation);
    gl.vertexAttribPointer(positionAttributeLocation, 2, gl.FLOAT, false, 0, 0);

    // 纹理坐标缓冲区
    const texCoordBuffer = gl.createBuffer();
    if (!texCoordBuffer) {
      throw new Error('Failed to create texture coordinate buffer');
    }
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

    this.resources = {
      canvas,
      gl,
      program,
      vao,
      positionBuffer,
      texCoordBuffer,
    };

    console.log('[WEBGL_MANAGER] WebGL 初始化完成');
    return this.resources;
  }

  // 获取 WebGL 资源
  public getResources(): WebGLResources | undefined {
    return this.resources;
  }

  // 设置矩形位置
  public setRectangle(
    x: number,
    y: number,
    width: number,
    height: number
  ): void {
    if (!this.resources) {
      console.error('[WEBGL_MANAGER] WebGL 资源未初始化');
      return;
    }

    const { gl, positionBuffer } = this.resources;
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

  // 准备渲染（设置视口、清除画布等）
  public prepareRender(): void {
    if (!this.resources) {
      console.error('[WEBGL_MANAGER] WebGL 资源未初始化');
      return;
    }

    const { canvas, gl } = this.resources;

    // 确保 canvas 尺寸正确
    webglUtils.resizeCanvasToDisplaySize(canvas);

    // 设置视口
    gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);

    // 清除 canvas
    gl.clearColor(0, 0, 0, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);

    // 使用程序
    gl.useProgram(this.resources.program);

    // 绑定 VAO
    gl.bindVertexArray(this.resources.vao);
  }

  // 设置 uniforms
  public setUniforms(): void {
    if (!this.resources) {
      console.error('[WEBGL_MANAGER] WebGL 资源未初始化');
      return;
    }

    const { gl, program } = this.resources;
    const resolutionLocation = gl.getUniformLocation(program, 'u_resolution');
    const imageLocation = gl.getUniformLocation(program, 'u_image');

    gl.uniform2f(resolutionLocation, gl.canvas.width, gl.canvas.height);
    gl.uniform1i(imageLocation, 0);
  }

  // 绘制
  public draw(): void {
    if (!this.resources) {
      console.error('[WEBGL_MANAGER] WebGL 资源未初始化');
      return;
    }

    const { gl } = this.resources;
    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }

  // 清理资源
  public cleanup(): void {
    if (this.resources) {
      const { gl, program, vao, positionBuffer, texCoordBuffer } =
        this.resources;

      if (program) gl.deleteProgram(program);
      if (vao) gl.deleteVertexArray(vao);
      if (positionBuffer) gl.deleteBuffer(positionBuffer);
      if (texCoordBuffer) gl.deleteBuffer(texCoordBuffer);

      this.resources = undefined;
      console.log('[WEBGL_MANAGER] WebGL 资源已清理');
    }
  }
}

// 导出单例实例
export const webglManager = new WebGLManager();
