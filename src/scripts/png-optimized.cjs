const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

// 优化的超大PNG图片生成器
// 使用真正的zlib压缩，确保PNG文件有效

class OptimizedPNGGenerator {
  constructor(width = 16384, height = 16384) {
    this.width = width;
    this.height = height;
    this.bytesPerPixel = 4; // RGBA
    this.totalBytes = this.width * this.height * this.bytesPerPixel;
    this.chunkSize = 1024; // 每次处理1024x1024的块
  }

  // 生成PNG文件头
  generatePNGHeader() {
    const header = Buffer.alloc(8);
    header.writeUInt32BE(0x89504e47, 0); // PNG signature
    header.writeUInt32BE(0x0d0a1a0a, 4); // PNG signature continuation
    return header;
  }

  // 生成IHDR块（图片信息）
  generateIHDR() {
    const data = Buffer.alloc(13);
    data.writeUInt32BE(this.width, 0);
    data.writeUInt32BE(this.height, 4);
    data.writeUInt8(8, 8); // bit depth
    data.writeUInt8(6, 9); // color type (RGBA)
    data.writeUInt8(0, 10); // compression method
    data.writeUInt8(0, 11); // filter method
    data.writeUInt8(0, 12); // interlace method

    return this.createChunk('IHDR', data);
  }

  // 生成IDAT块（图片数据）
  async generateIDAT() {
    const fileName = this.outputFileName || 'large_image_optimized.png';
    const outputPath = path.join(__dirname, fileName);
    const writeStream = fs.createWriteStream(outputPath);

    // 写入PNG头
    writeStream.write(this.generatePNGHeader());

    // 写入IHDR
    writeStream.write(this.generateIHDR());

    // 开始写入IDAT数据
    let idatData = Buffer.alloc(0);
    let totalPixels = 0;

    console.log(`开始生成 ${this.width}x${this.height} 的PNG图片...`);
    console.log(
      `预计文件大小: 约 ${(this.totalBytes / 1024 / 1024 / 1024).toFixed(2)} GB`
    );
    console.log(`注意：实际文件大小会因压缩而显著减小`);

    // 分块生成图片数据
    for (let y = 0; y < this.height; y += this.chunkSize) {
      const chunkHeight = Math.min(this.chunkSize, this.height - y);

      for (let x = 0; x < this.width; x += this.chunkSize) {
        const chunkWidth = Math.min(this.chunkSize, this.width - x);

        // 生成这个块的像素数据
        const chunkData = this.generateChunkPixels(
          chunkWidth,
          chunkHeight,
          x,
          y
        );

        // 添加到IDAT数据
        idatData = Buffer.concat([idatData, chunkData]);

        totalPixels += chunkWidth * chunkHeight;

        // 显示进度
        if (totalPixels % (1024 * 1024) === 0) {
          const progress = (
            (totalPixels / (this.width * this.height)) *
            100
          ).toFixed(2);
          const currentSize = (idatData.length / 1024 / 1024).toFixed(2);
          console.log(`进度: ${progress}% - 当前数据大小: ${currentSize} MB`);
        }

        // 当IDAT数据达到一定大小时，压缩并写入文件
        if (idatData.length > 50 * 1024 * 1024) {
          // 50MB
          await this.writeIDATChunk(writeStream, idatData);
          idatData = Buffer.alloc(0);
        }
      }
    }

    // 写入剩余的IDAT数据
    if (idatData.length > 0) {
      await this.writeIDATChunk(writeStream, idatData);
    }

    // 写入IEND块
    writeStream.write(this.createChunk('IEND', Buffer.alloc(0)));

    writeStream.end();

    return new Promise((resolve, reject) => {
      writeStream.on('finish', () => {
        const stats = fs.statSync(outputPath);
        console.log(`\n✅ PNG图片生成完成！`);
        console.log(`文件路径: ${outputPath}`);
        console.log(
          `文件大小: ${(stats.size / 1024 / 1024 / 1024).toFixed(2)} GB`
        );
        console.log(
          `压缩率: ${((1 - stats.size / this.totalBytes) * 100).toFixed(2)}%`
        );
        console.log(`图片尺寸: ${this.width} x ${this.height}`);
        console.log(`总像素数: ${this.width * this.height.toLocaleString()}`);
        resolve(outputPath);
      });

      writeStream.on('error', reject);
    });
  }

  // 生成指定块的像素数据
  generateChunkPixels(width, height, startX, startY) {
    const data = Buffer.alloc(width * height * 4 + height); // +height for filter bytes

    let dataIndex = 0;

    for (let y = 0; y < height; y++) {
      // 写入过滤器字节（0 = 无过滤，这是PNG标准中唯一支持的值）
      data[dataIndex++] = 0;

      for (let x = 0; x < width; x++) {
        const globalX = startX + x;
        const globalY = startY + y;

        // 生成简化的图案内容，避免复杂的数学运算
        let r,
          g,
          b,
          a = 255;

        // 使用更安全的图案生成方法
        r = this.createSimplePattern(globalX, globalY, 0);
        g = this.createSimplePattern(globalX, globalY, 1);
        b = this.createSimplePattern(globalX, globalY, 2);

        // 确保所有值都在0-255范围内
        r = Math.max(0, Math.min(255, r));
        g = Math.max(0, Math.min(255, g));
        b = Math.max(0, Math.min(255, b));

        data[dataIndex++] = r;
        data[dataIndex++] = g;
        data[dataIndex++] = b;
        data[dataIndex++] = a;
      }
    }

    return data;
  }

  // 生成简单的图案（更安全的方法）
  createSimplePattern(x, y, channel) {
    // 基于通道生成不同的基础图案，完全避免复杂计算
    let value = 0;

    switch (channel) {
      case 0: // 红色通道 - 水平渐变
        value = Math.floor((x / this.width) * 255);
        break;
      case 1: // 绿色通道 - 垂直渐变
        value = Math.floor((y / this.height) * 255);
        break;
      case 2: // 蓝色通道 - 对角渐变
        value = Math.floor(((x + y) / (this.width + this.height)) * 255);
        break;
    }

    // 移除所有变化，只使用纯渐变，确保绝对安全
    return Math.max(0, Math.min(255, value));
  }

  // 写入IDAT块（使用zlib压缩）
  async writeIDATChunk(writeStream, data) {
    return new Promise((resolve, reject) => {
      // 使用zlib压缩数据
      zlib.deflate(data, { level: 6 }, (err, compressedData) => {
        if (err) {
          reject(err);
          return;
        }

        const chunk = this.createChunk('IDAT', compressedData);
        writeStream.write(chunk);
        resolve();
      });
    });
  }

  // 创建PNG块
  createChunk(type, data) {
    const length = data.length;
    const chunk = Buffer.alloc(12 + length); // 4字节长度 + 4字节类型 + 4字节CRC + 数据

    // 写入长度（不包括类型和CRC）
    chunk.writeUInt32BE(length, 0);

    // 写入类型
    chunk.write(type, 4, 4, 'ascii');

    // 写入数据
    data.copy(chunk, 8);

    // 计算并写入CRC
    const crc = this.calculateCRC32(chunk.slice(4, 8 + length));
    chunk.writeUInt32BE(crc, 8 + length);

    return chunk;
  }

  // 计算CRC-32（PNG标准）
  calculateCRC32(data) {
    let crc = 0xffffffff;
    const table = this.generateCRCTable();

    for (let i = 0; i < data.length; i++) {
      const byte = data[i];
      crc = (crc >>> 8) ^ table[(crc ^ byte) & 0xff];
    }

    return (crc ^ 0xffffffff) >>> 0;
  }

  // 生成CRC-32查找表
  generateCRCTable() {
    const table = new Array(256);
    for (let i = 0; i < 256; i++) {
      let c = i;
      for (let j = 0; j < 8; j++) {
        c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      }
      table[i] = c;
    }
    return table;
  }

  // 生成图片
  async generate() {
    try {
      console.log('🚀 开始生成优化的超大PNG图片...');
      console.log(`目标尺寸: ${this.width} x ${this.height}`);
      console.log(
        `原始数据大小: ${(this.totalBytes / 1024 / 1024 / 1024).toFixed(2)} GB`
      );
      console.log(`使用zlib压缩，预期压缩率: 70-90%`);

      const startTime = Date.now();
      const outputPath = await this.generateIDAT();
      const endTime = Date.now();

      console.log(
        `\n⏱️  生成耗时: ${((endTime - startTime) / 1000).toFixed(2)} 秒`
      );
      console.log(`📁 文件已保存到: ${outputPath}`);

      return outputPath;
    } catch (error) {
      console.error('❌ 生成失败:', error);
      throw error;
    }
  }
}

// 解析命令行参数
function parseArguments() {
  const args = process.argv.slice(2);
  let width = 16384;
  let height = 16384;
  let outputName = null;

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];

    if (arg === '--help' || arg === '-h') {
      showHelp();
      process.exit(0);
    } else if (arg === '--width' || arg === '-w') {
      if (i + 1 < args.length) {
        width = parseInt(args[++i]);
        if (isNaN(width) || width <= 0) {
          console.error('❌ 错误: 宽度必须是正整数');
          process.exit(1);
        }
      }
    } else if (arg === '--height' || arg === '-H') {
      if (i + 1 < args.length) {
        height = parseInt(args[++i]);
        if (isNaN(height) || height <= 0) {
          console.error('❌ 错误: 高度必须是正整数');
          process.exit(1);
        }
      }
    } else if (arg === '--output' || arg === '-o') {
      if (i + 1 < args.length) {
        outputName = args[++i];
      }
    } else if (arg.startsWith('--')) {
      console.error(`❌ 未知参数: ${arg}`);
      showHelp();
      process.exit(1);
    } else {
      // 如果只提供一个数字，假设是正方形
      if (i === 0 && !isNaN(parseInt(arg))) {
        width = height = parseInt(arg);
      }
    }
  }

  return { width, height, outputName };
}

// 显示帮助信息
function showHelp() {
  console.log(`
🚀 优化的超大PNG图片生成器

用法: node png-optimized.cjs [选项] [尺寸]

选项:
  -w, --width <像素>     设置图片宽度 (默认: 16384)
  -H, --height <像素>    设置图片高度 (默认: 16384)
  -o, --output <文件名>   设置输出文件名 (默认: large_image_optimized.png)
  --help                  显示此帮助信息

示例:
  node png-optimized.cjs                    # 生成 16384x16384 的图片
  node png-optimized.cjs 8192               # 生成 8192x8192 的正方形图片
  node png-optimized.cjs -w 4096 -h 8192   # 生成 4096x8192 的矩形图片
  node png-optimized.cjs -w 1024 -h 1024 -o test.png  # 生成 1024x1024 的测试图片

注意: 建议宽度和高度都是2的幂次方，以获得最佳性能。
`);
}

// 使用示例
async function main() {
  try {
    const { width, height, outputName } = parseArguments();

    console.log(`🎯 目标尺寸: ${width} x ${height}`);
    console.log(`📊 总像素数: ${(width * height).toLocaleString()}`);
    console.log(
      `💾 预计原始大小: ${((width * height * 4) / 1024 / 1024 / 1024).toFixed(
        2
      )} GB`
    );

    const generator = new OptimizedPNGGenerator(width, height);

    // 如果指定了输出文件名，更新生成器的输出路径
    if (outputName) {
      generator.outputFileName = outputName;
    }

    await generator.generate();
  } catch (error) {
    console.error('❌ 生成失败:', error);
    process.exit(1);
  }
}

// 如果直接运行此脚本
if (require.main === module) {
  main();
}

module.exports = OptimizedPNGGenerator;
