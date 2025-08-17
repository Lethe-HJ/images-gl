好的！让我详细解释一下这个图片加载和显示的完整流程：

## �� 整体数据流程

```
PNG 文件 → Rust 解码 → 前端接收 → WebGL 纹理 → Canvas 显示
```

## �� Rust 端（后端）

### 1. **图片解码**

```rust
let img = image::open(file_path).map_err(|e| format!("图片解码失败: {}", e))?;
```

- 使用 `image` crate 打开 PNG 文件
- 自动解码 PNG 格式，转换为内存中的图片对象
- 如果失败，返回错误信息

### 2. **获取尺寸信息**

```rust
let (width, height) = img.dimensions();
```

- 从解码后的图片中提取宽度和高度
- 你的图片是 6000x3818 像素

### 3. **格式转换**

```rust
let rgba_img = img.to_rgba8();
let pixels = rgba_img.into_raw();
```

- 将图片转换为 RGBA8 格式（每个像素 4 字节）
- `into_raw()` 提取原始字节数据
- 总字节数：6000 × 3818 × 4 = 91,632,000 字节

### 4. **构建数据包**

```rust
let mut data = Vec::with_capacity(8 + pixels.len());
data.extend_from_slice(&width.to_be_bytes());  // 前4字节：宽度
data.extend_from_slice(&height.to_be_bytes()); // 中间4字节：高度
data.extend_from_slice(&pixels);              // 剩余字节：像素数据
```

**数据包结构：**

```
[宽度4字节][高度4字节][像素数据...]
[6000][3818][RGBA像素1][RGBA像素2]...
```

## 🌐 前端（TypeScript）

### 1. **接收数据**

```typescript
const imageBuffer = await invoke("read_file");
```

- 调用 Rust 函数，接收 ArrayBuffer 数据

### 2. **解析尺寸信息**

```typescript
const data = new Uint8Array(imageBuffer);
const width = new DataView(data.buffer).getUint32(0); // 读取前4字节
const height = new DataView(data.buffer).getUint32(4); // 读取中间4字节
```

- 使用 `DataView` 读取大端序的 32 位整数
- 从字节数组中提取宽度和高度

### 3. **提取像素数据**

```typescript
const pixels = new Uint8Array(data.slice(8));
```

- 跳过前 8 字节的尺寸信息
- 获取剩余的像素数据

### 4. **创建纹理**

```typescript
// 创建 ImageData 对象
const imageData = new ImageData(new Uint8ClampedArray(pixels), width, height);

// 通过临时 canvas 创建纹理
const tempCanvas = document.createElement("canvas");
tempCanvas.width = width;
tempCanvas.height = height;
const tempCtx = tempCanvas.getContext("2d");
tempCtx.putImageData(imageData, 0, 0);

// 上传到 WebGL
gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, tempCanvas);
```

## 🎨 WebGL 渲染

### 1. **初始化资源**

- 创建着色器程序
- 设置顶点缓冲区
- 配置纹理坐标

### 2. **视图管理**

```typescript
// 计算初始缩放比例，使图片完全显示
const scaleX = canvasWidth / imageWidth; // 798 / 6000 = 0.133
const scaleY = canvasHeight / imageHeight; // 570 / 3818 = 0.149
const scale = Math.min(scaleX, scaleY, 1.0); // 取最小值 0.133
```

### 3. **绘制**

- 根据缩放比例计算绘制位置
- 使用 WebGL 绘制图片到 canvas

## �� 关键技术点

### 1. **大端序（Big Endian）**

```rust
width.to_be_bytes()  // 转换为大端序字节数组
```

- 确保跨平台一致性
- 前端使用 `DataView.getUint32()` 正确读取

### 2. **内存布局**

```
[宽度4字节][高度4字节][像素数据...]
```

- 固定头部结构
- 便于前端解析

### 3. **性能优化**

- Rust 端完成解码，避免前端解码开销
- 直接传输原始像素数据
- 使用 `Uint8Array` 避免类型转换

### 4. **错误处理**

- Rust 端详细的错误信息
- 前端 try-catch 捕获异常
- 用户友好的错误提示

## �� 数据大小分析

- **原始 PNG**：37MB（压缩格式）
- **解码后**：91.6MB（6000×3818×4 字节）
- **传输效率**：虽然数据变大，但避免了前端解码的 CPU 开销

这种架构的优势是：

1. **后端解码**：利用 Rust 的高性能图片处理
2. **直接传输**：避免 JSON 序列化开销
3. **前端渲染**：专注于 WebGL 渲染和用户交互
4. **内存管理**：合理的内存分配和释放

现在你的图片应该能正常显示，并且支持滚轮缩放了！
