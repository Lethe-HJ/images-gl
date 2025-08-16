/**
 * Wrapped logging function.
 * @param msg The message to log.
 */
function error(msg: string): void {
  if (window.console) {
    if (window.console.error) {
      window.console.error(msg);
    } else if (window.console.log) {
      window.console.log(msg);
    }
  }
}

const errorRE = /ERROR:\s*\d+:(\d+)/gi;

/**
 * Add line numbers to error messages
 */
function addLineNumbersWithError(src: string, log: string = ''): string {
  // Note: Error message formats are not defined by any spec so this may or may not work.
  const matches = [...log.matchAll(errorRE)];
  const lineNoToErrorMap = new Map(matches.map((m, ndx) => {
    const lineNo = parseInt(m[1]);
    const next = matches[ndx + 1];
    const end = next ? next.index : log.length;
    const msg = log.substring(m.index, end);
    return [lineNo - 1, msg];
  }));
  return src.split('\n').map((line, lineNo) => {
    const err = lineNoToErrorMap.get(lineNo);
    return `${lineNo + 1}: ${line}${err ? `\n\n^^^ ${err}` : ''}`;
  }).join('\n');
}

/**
 * Convert WebGL enum to string
 */
function glEnumToString(gl: WebGLRenderingContext, value: number): string {
  for (const key in gl) {
    if (gl[key as keyof WebGLRenderingContext] === value) {
      return key;
    }
  }
  return `0x${value.toString(16)}`;
}

/**
 * Error Callback type
 */
export type ErrorCallback = (msg: string) => void;

/**
 * Loads a shader.
 * @param gl The WebGLRenderingContext to use.
 * @param shaderSource The shader source.
 * @param shaderType The type of shader.
 * @param opt_errorCallback callback for errors.
 * @return The created shader.
 */
function loadShader(
  gl: WebGLRenderingContext, 
  shaderSource: string, 
  shaderType: number, 
  opt_errorCallback?: ErrorCallback
): WebGLShader | null {
  const errFn = opt_errorCallback || error;
  // Create the shader object
  const shader = gl.createShader(shaderType);
  if (!shader) return null;

  // Load the shader source
  gl.shaderSource(shader, shaderSource);

  // Compile the shader
  gl.compileShader(shader);

  // Check the compile status
  const compiled = gl.getShaderParameter(shader, gl.COMPILE_STATUS);
  if (!compiled) {
    // Something went wrong during compilation; get the error
    const lastError = gl.getShaderInfoLog(shader);
    errFn(`Error compiling shader: ${lastError}\n${addLineNumbersWithError(shaderSource, lastError || '')}`);
    gl.deleteShader(shader);
    return null;
  }

  return shader;
}

/**
 * Creates a program, attaches shaders, binds attrib locations, links the
 * program and calls useProgram.
 * @param gl The WebGLRenderingContext to use
 * @param shaders The shaders to attach
 * @param opt_attribs An array of attribs names. Locations will be assigned by index if not passed in
 * @param opt_locations The locations for the. A parallel array to opt_attribs letting you assign locations.
 * @param opt_errorCallback callback for errors. By default it just prints an error to the console
 *        on error. If you want something else pass an callback. It's passed an error message.
 */
function createProgram(
  gl: WebGLRenderingContext,
  shaders: WebGLShader[],
  opt_attribs?: string[],
  opt_locations?: number[],
  opt_errorCallback?: ErrorCallback
): WebGLProgram | null {
  const errFn = opt_errorCallback || error;
  const program = gl.createProgram();
  if (!program) return null;

  shaders.forEach(function(shader) {
    gl.attachShader(program, shader);
  });
  
  if (opt_attribs) {
    opt_attribs.forEach(function(attrib, ndx) {
      gl.bindAttribLocation(
        program,
        opt_locations ? opt_locations[ndx] : ndx,
        attrib);
    });
  }
  
  gl.linkProgram(program);

  // Check the link status
  const linked = gl.getProgramParameter(program, gl.LINK_STATUS);
  if (!linked) {
    // something went wrong with the link
    const lastError = gl.getProgramInfoLog(program);
    errFn(`Error in program linking: ${lastError}\n${
      shaders.map(shader => {
        const src = addLineNumbersWithError(gl.getShaderSource(shader) || '');
        const type = gl.getShaderParameter(shader, gl.SHADER_TYPE);
        return `${glEnumToString(gl, type)}:\n${src}`;
      }).join('\n')
    }`);

    gl.deleteProgram(program);
    return null;
  }
  return program;
}

/**
 * Loads a shader from a script tag.
 * @param gl The WebGLRenderingContext to use.
 * @param scriptId The id of the script tag.
 * @param opt_shaderType The type of shader. If not passed in it will
 *     be derived from the type of the script tag.
 * @param opt_errorCallback callback for errors.
 * @return The created shader.
 */
function createShaderFromScript(
  gl: WebGLRenderingContext,
  scriptId: string,
  opt_shaderType?: number,
  opt_errorCallback?: ErrorCallback
): WebGLShader | null {
  let shaderSource = "";
  let shaderType: number;
  
  const shaderScript = document.getElementById(scriptId) as HTMLScriptElement;
  if (!shaderScript) {
    throw new Error("*** Error: unknown script element " + scriptId);
  }
  
  shaderSource = shaderScript.text || '';

  if (!opt_shaderType) {
    if (shaderScript.type === "x-shader/x-vertex") {
      shaderType = gl.VERTEX_SHADER;
    } else if (shaderScript.type === "x-shader/x-fragment") {
      shaderType = gl.FRAGMENT_SHADER;
    } else {
      throw new Error("*** Error: unknown shader type");
    }
  } else {
    shaderType = opt_shaderType;
  }

  return loadShader(
    gl, shaderSource, shaderType, opt_errorCallback);
}

const defaultShaderType = [
  "VERTEX_SHADER",
  "FRAGMENT_SHADER",
] as const;

/**
 * Creates a program from 2 script tags.
 *
 * @param gl The WebGLRenderingContext to use.
 * @param shaderScriptIds Array of ids of the script
 *        tags for the shaders. The first is assumed to be the
 *        vertex shader, the second the fragment shader.
 * @param opt_attribs An array of attribs names. Locations will be assigned by index if not passed in
 * @param opt_locations The locations for the. A parallel array to opt_attribs letting you assign locations.
 * @param opt_errorCallback callback for errors. By default it just prints an error to the console
 *        on error. If you want something else pass an callback. It's passed an error message.
 * @return The created program.
 */
function createProgramFromScripts(
  gl: WebGLRenderingContext,
  shaderScriptIds: string[],
  opt_attribs?: string[],
  opt_locations?: number[],
  opt_errorCallback?: ErrorCallback
): WebGLProgram | null {
  const shaders: WebGLShader[] = [];
  for (let ii = 0; ii < shaderScriptIds.length; ++ii) {
    const shader = createShaderFromScript(
      gl, shaderScriptIds[ii], gl[defaultShaderType[ii] as keyof WebGLRenderingContext] as number, opt_errorCallback);
    if (shader) {
      shaders.push(shader);
    }
  }
  return createProgram(gl, shaders, opt_attribs, opt_locations, opt_errorCallback);
}

/**
 * Creates a program from 2 sources.
 *
 * @param gl The WebGLRenderingContext to use.
 * @param shaderSources Array of sources for the
 *        shaders. The first is assumed to be the vertex shader,
 *        the second the fragment shader.
 * @param opt_attribs An array of attribs names. Locations will be assigned by index if not passed in
 * @param opt_locations The locations for the. A parallel array to opt_attribs letting you assign locations.
 * @param opt_errorCallback callback for errors. By default it just prints an error to the console
 *        on error. If you want something else pass an callback. It's passed an error message.
 * @return The created program.
 */
function createProgramFromSources(
  gl: WebGLRenderingContext,
  shaderSources: string[],
  opt_attribs?: string[],
  opt_locations?: number[],
  opt_errorCallback?: ErrorCallback
): WebGLProgram | null {
  const shaders: WebGLShader[] = [];
  for (let ii = 0; ii < shaderSources.length; ++ii) {
    const shader = loadShader(
      gl, shaderSources[ii], gl[defaultShaderType[ii] as keyof WebGLRenderingContext] as number, opt_errorCallback);
    if (shader) {
      shaders.push(shader);
    }
  }
  return createProgram(gl, shaders, opt_attribs, opt_locations, opt_errorCallback);
}

/**
 * Resize a canvas to match the size its displayed.
 * @param canvas The canvas to resize.
 * @param multiplier amount to multiply by.
 *    Pass in window.devicePixelRatio for native pixels.
 * @return true if the canvas was resized.
 */
function resizeCanvasToDisplaySize(canvas: HTMLCanvasElement, multiplier?: number): boolean {
  multiplier = multiplier || 1;
  const width = canvas.clientWidth * multiplier | 0;
  const height = canvas.clientHeight * multiplier | 0;
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
    return true;
  }
  return false;
}

// Also export the default object
export default {
    createProgram,
    createProgramFromScripts,
    createProgramFromSources,
    resizeCanvasToDisplaySize,
  };