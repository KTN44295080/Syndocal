import { normalizeFxSource, readIsfMetadata } from "./fxCatalogModel";

export interface FxThumbnail { url?: string; rendered: boolean; reason?: string }
const cache = new Map<string, FxThumbnail>();
let canvas: HTMLCanvasElement | undefined;
let context: WebGLRenderingContext | null | undefined;
const WIDTH = 160;
const HEIGHT = 90;

function standardImage(): Uint8Array {
  const pixels = new Uint8Array(WIDTH * HEIGHT * 4);
  for (let y = 0; y < HEIGHT; y++) for (let x = 0; x < WIDTH; x++) {
    const offset = (y * WIDTH + x) * 4;
    const checker = (Math.floor(x / 16) + Math.floor(y / 15)) % 2;
    const circle = Math.hypot(x - WIDTH * 0.64, y - HEIGHT * 0.55) < HEIGHT * 0.28;
    pixels.set(circle ? [240, 190, 40, 255] : [x / WIDTH * 230 + checker * 25, y / HEIGHT * 200 + 25, 210 - x / WIDTH * 150, 255], offset);
  }
  return pixels;
}

/** One still frame, one shared offscreen context, bounded cache; never touches a
 * layer or output. Complex/multipass shaders use the clearly labelled fallback. */
export function renderFxThumbnail(source: string): FxThumbnail {
  const key = normalizeFxSource(source);
  const known = cache.get(key);
  if (known) return known;
  let result: FxThumbnail;
  try { result = render(key); }
  catch (error) { result = { rendered: false, reason: error instanceof Error ? error.message : String(error) }; }
  if (cache.size >= 128) cache.delete(cache.keys().next().value!);
  cache.set(key, result);
  return result;
}

function render(source: string): FxThumbnail {
  if (typeof document === "undefined") return { rendered: false, reason: "No canvas" };
  const metadata = readIsfMetadata(source);
  if (source.length > 32768 || /\b(?:for|while|do)\b/.test(source.replace(/\/\*[\s\S]*?\*\/|\/\/[^\n]*/g, ""))) return { rendered: false, reason: "Complex shader" };
  if (Array.isArray(metadata.PASSES) && (metadata.PASSES.length > 1 || metadata.PASSES.some((pass) => pass && typeof pass === "object" && ("TARGET" in pass || "PERSISTENT" in pass)))) return { rendered: false, reason: "Multipass shader" };
  if (!canvas) { canvas = document.createElement("canvas"); canvas.width = WIDTH; canvas.height = HEIGHT; context = canvas.getContext("webgl", { preserveDrawingBuffer: true, antialias: false, alpha: false }); }
  const gl = context;
  if (!gl || gl.isContextLost()) return { rendered: false, reason: "WebGL unavailable" };
  const shaders: WebGLShader[] = [];
  let program: WebGLProgram | null = null;
  let buffer: WebGLBuffer | null = null;
  let texture: WebGLTexture | null = null;
  try {
    const inputs = Array.isArray(metadata.INPUTS) ? metadata.INPUTS as Record<string, unknown>[] : [];
    const types: Record<string, string> = { image: "sampler2D", float: "float", long: "int", bool: "bool", event: "bool", point2D: "vec2", color: "vec4" };
    if (inputs.some((input) => !types[String(input.TYPE)] || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(String(input.NAME)))) return { rendered: false, reason: "Unsupported input" };
    const declarations = inputs.map((input) => `uniform ${types[String(input.TYPE)]} ${input.NAME};`).join("\n");
    const body = source.replace(/\/\*[\s\S]*?\*\//, "").replace(/^\s*#version[^\n]*/gm, "");
    const fragment = `precision highp float;\nvarying vec2 isf_FragNormCoord;\nuniform float TIME;\nuniform float TIMEDELTA;\nuniform int FRAMEINDEX;\nuniform vec2 RENDERSIZE;\n${declarations}\n#define IMG_NORM_PIXEL(img, uv) texture2D(img, clamp(uv, 0.0, 1.0))\n#define IMG_THIS_PIXEL(img) IMG_NORM_PIXEL(img, isf_FragNormCoord)\n#define IMG_PIXEL(img, uv) IMG_NORM_PIXEL(img, (uv) / RENDERSIZE)\n#define IMG_SIZE(img) RENDERSIZE\n${body}`;
    const compile = (kind: number, text: string) => {
      const shader = gl.createShader(kind);
      if (!shader) throw new Error("Shader allocation failed");
      shaders.push(shader); gl.shaderSource(shader, text); gl.compileShader(shader);
      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader) ?? "Thumbnail compilation failed");
      return shader;
    };
    const vertex = compile(gl.VERTEX_SHADER, "attribute vec2 position; varying vec2 isf_FragNormCoord; void main(){ isf_FragNormCoord = position * 0.5 + 0.5; gl_Position = vec4(position, 0.0, 1.0); }");
    const pixel = compile(gl.FRAGMENT_SHADER, fragment);
    program = gl.createProgram();
    if (!program) throw new Error("Program allocation failed");
    gl.attachShader(program, vertex); gl.attachShader(program, pixel); gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error("Thumbnail link failed");
    gl.useProgram(program);
    buffer = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    const position = gl.getAttribLocation(program, "position");
    gl.enableVertexAttribArray(position); gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
    texture = gl.createTexture(); gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, WIDTH, HEIGHT, 0, gl.RGBA, gl.UNSIGNED_BYTE, standardImage());
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.uniform1f(gl.getUniformLocation(program, "TIME"), 0.25);
    gl.uniform1f(gl.getUniformLocation(program, "TIMEDELTA"), 1 / 60);
    gl.uniform1i(gl.getUniformLocation(program, "FRAMEINDEX"), 15);
    gl.uniform2f(gl.getUniformLocation(program, "RENDERSIZE"), WIDTH, HEIGHT);
    for (const input of inputs) {
      const location = gl.getUniformLocation(program, String(input.NAME));
      const values = Array.isArray(input.DEFAULT) ? input.DEFAULT.map(Number) : [typeof input.DEFAULT === "boolean" ? Number(input.DEFAULT) : Number(input.DEFAULT ?? 0)];
      if (!values.every(Number.isFinite)) return { rendered: false, reason: "Invalid default" };
      switch (input.TYPE) {
        case "image": gl.uniform1i(location, 0); break;
        case "long": case "bool": case "event": gl.uniform1i(location, Math.round(values[0])); break;
        case "point2D": gl.uniform2f(location, values[0] ?? 0.5, values[1] ?? 0.5); break;
        case "color": gl.uniform4f(location, values[0] ?? 1, values[1] ?? 1, values[2] ?? 1, values[3] ?? 1); break;
        default: gl.uniform1f(location, values[0]);
      }
    }
    gl.viewport(0, 0, WIDTH, HEIGHT); gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    if (gl.getError() !== gl.NO_ERROR) return { rendered: false, reason: "Thumbnail rendering unavailable" };
    return { url: canvas.toDataURL("image/png"), rendered: true };
  } finally {
    if (texture) gl.deleteTexture(texture);
    if (buffer) gl.deleteBuffer(buffer);
    if (program) gl.deleteProgram(program);
    for (const shader of shaders) gl.deleteShader(shader);
  }
}
