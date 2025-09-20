# 超大 PNG 图片生成器

这个目录包含了生成超大 PNG 图片的脚本，用于测试图片处理系统的性能。

## 文件说明

### 1. `png.js` - 基础版本

- 生成约 1GB 的 PNG 图片
- 使用简化的压缩算法
- 适合快速测试

### 2. `png-optimized.js` - 优化版本 ⭐ 推荐

- 使用真正的 zlib 压缩
- 生成有效的 PNG 文件
- 压缩率更高，文件更小

## 使用方法

### 直接运行

```bash
# 生成基础版本
node src/scripts/png.js

# 生成优化版本（推荐）
node src/scripts/png-optimized.js
```

### 作为模块使用

```javascript
const OptimizedPNGGenerator = require('./src/scripts/png-optimized.js');

const generator = new OptimizedPNGGenerator();
generator.generate().then(outputPath => {
  console.log('图片生成完成:', outputPath);
});
```

## 技术规格

### 图片参数

- **尺寸**: 16384 × 16384 像素
- **颜色模式**: RGBA (32 位)
- **原始数据大小**: 约 1GB
- **预期压缩后大小**: 100-300MB

### 生成策略

- **分块处理**: 1024×1024 像素块
- **内存优化**: 流式写入，避免内存溢出
- **进度显示**: 实时显示生成进度
- **压缩优化**: 使用 zlib 压缩算法

## 性能特点

### 优势

- ✅ 内存友好：分块处理，不会导致内存溢出
- ✅ 进度可见：实时显示生成进度
- ✅ 压缩高效：zlib 压缩，文件大小显著减小
- ✅ 格式标准：生成有效的 PNG 文件

### 注意事项

- ⚠️ 生成时间：根据硬件性能，可能需要几分钟到几十分钟
- ⚠️ 磁盘空间：确保有足够的磁盘空间（至少 2GB）
- ⚠️ 内存使用：虽然分块处理，但仍需要一定内存

## 自定义配置

你可以修改脚本中的参数来调整生成的图片：

```javascript
class OptimizedPNGGenerator {
  constructor() {
    this.width = 16384; // 修改宽度
    this.height = 16384; // 修改高度
    this.chunkSize = 1024; // 修改分块大小
  }
}
```

## 测试用途

生成的超大 PNG 图片可以用于：

1. **性能测试**: 测试图片加载和处理性能
2. **内存测试**: 测试大图片的内存管理
3. **分块测试**: 测试图片分块加载系统
4. **压缩测试**: 测试不同压缩算法的效果

## 故障排除

### 常见问题

**Q: 生成过程中内存不足**
A: 减小 `chunkSize` 参数，或增加系统内存

**Q: 生成的文件无法打开**
A: 使用 `png-optimized.js` 版本，它生成有效的 PNG 文件

**Q: 生成时间过长**
A: 减小图片尺寸，或使用更快的存储设备

**Q: 磁盘空间不足**
A: 确保有足够的磁盘空间，或减小图片尺寸
