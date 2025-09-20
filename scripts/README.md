# EXR到PNG转换工具

这个目录包含了将EXR格式图像转换为PNG格式的Node.js脚本。

## 安装依赖

```bash
npm install parse-exr sharp
```

## 脚本说明

### 1. `exr-to-png.js` - 主要转换脚本

这是一个功能完整的EXR到PNG转换工具，支持：

- 单个文件转换
- 批量转换
- 尺寸调整
- 质量控制
- 色调映射（HDR到SDR转换）

#### 使用方法

**单个文件转换：**

```bash
node exr-to-png.js input.exr output.png
```

**带选项的转换：**

```bash
# 调整输出尺寸
node exr-to-png.js input.exr output.png --width 1920 --height 1080

# 设置PNG质量
node exr-to-png.js input.exr output.png --quality 95

# 禁用色调映射
node exr-to-png.js input.exr output.png --no-tone-mapping
```

**批量转换：**

```bash
# 转换整个目录
node exr-to-png.js --batch ./exr-files ./png-files

# 批量转换并设置选项
node exr-to-png.js --batch ./exr-files ./png-files --quality 95 --width 1920
```

#### 选项说明

- `--width <number>`: 输出图像宽度
- `--height <number>`: 输出图像高度
- `--quality <number>`: PNG质量 (1-100, 默认90)
- `--no-tone-mapping`: 禁用色调映射
- `--batch`: 启用批量转换模式

### 2. `test-exr-conversion.js` - 测试脚本

用于测试转换功能的简单脚本。

**使用方法：**

```bash
# 将test.exr文件放在scripts目录下
node test-exr-conversion.js
```

## 功能特性

### 色调映射 (Tone Mapping)

- 自动将HDR (High Dynamic Range) 值转换为SDR (Standard Dynamic Range)
- 支持线性色调映射算法
- 可选择性禁用

### 图像处理

- 使用Sharp库进行高质量的图像处理
- 支持尺寸调整
- PNG压缩优化

### 错误处理

- 详细的错误信息和日志
- 文件存在性检查
- 格式验证

## 技术细节

### 依赖库

- **parse-exr**: EXR文件解析器，基于Three.js实现
- **sharp**: 高性能图像处理库

### 数据流程

```
EXR文件 → parseEXR解析 → 色调映射 → Sharp处理 → PNG输出
```

### 内存管理

- 流式处理大文件
- 自动内存清理
- 缓冲区优化

## 示例

### 基本转换

```bash
cd scripts
node exr-to-png.js ../sample.exr ../output.png
```

### 批量处理

```bash
cd scripts
mkdir -p ../output-png
node exr-to-png.js --batch ../exr-files ../output-png --quality 90
```

### 在代码中使用

```javascript
const { convertEXRtoPNG } = require('./exr-to-png.js');

// 转换单个文件
const result = await convertEXRtoPNG('input.exr', 'output.png', {
  width: 1920,
  height: 1080,
  quality: 95,
});

if (result.success) {
  console.log('转换成功！');
} else {
  console.error('转换失败:', result.error);
}
```

## 注意事项

1. **文件格式**: 确保输入文件是有效的EXR格式
2. **内存使用**: 大尺寸EXR文件可能需要较多内存
3. **色调映射**: 默认启用色调映射，适合大多数HDR图像
4. **输出质量**: PNG质量设置影响文件大小和转换时间

## 故障排除

### 常见问题

**"parseEXR is not a function"**

- 确保已安装 `parse-exr` 包
- 检查Node.js版本兼容性

**内存不足错误**

- 减少同时处理的文件数量
- 考虑分批处理大文件

**色调映射效果不理想**

- 使用 `--no-tone-mapping` 选项
- 手动调整输入数据范围

## 许可证

本项目使用MIT许可证。
