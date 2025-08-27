const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

// 完全简化的PNG生成器 - 只用于测试格式正确性
class SimplePNGGenerator {
  constructor(width = 1024, height = 1024) {
    this.width = width;
    this.height = height;
    this.bytesPerPixel = 4; // RGBA
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

    return this.createChunk("IHDR", data);
  }

  // 生成IDAT块（图片数据）
  async generateIDAT() {
    const outputPath = path.join(__dirname, "simple_test.png");
    const writeStream = fs.createWriteStream(outputPath);

    // 写入PNG头
    writeStream.write(this.generatePNGHeader());

    // 写入IHDR
    writeStream.write(this.generateIHDR());

    // 生成图片数据
    const imageData = this.generateImageData();

    // 压缩并写入IDAT
    await this.writeIDATChunk(writeStream, imageData);

    // 写入IEND块
    writeStream.write(this.createChunk("IEND", Buffer.alloc(0)));

    writeStream.end();

    return new Promise((resolve, reject) => {
      writeStream.on("finish", () => {
        const stats = fs.statSync(outputPath);
        console.log(`\n✅ 简单PNG图片生成完成！`);
        console.log(`文件路径: ${outputPath}`);
        console.log(`文件大小: ${(stats.size / 1024).toFixed(2)} KB`);
        console.log(`图片尺寸: ${this.width} x ${this.height}`);
        resolve(outputPath);
      });

      writeStream.on("error", reject);
    });
  }

  // 生成图片数据 - 完全简化的渐变
  generateImageData() {
    const data = Buffer.alloc(this.height * (1 + this.width * 4)); // 每行：1字节过滤器 + width * 4字节像素
    let dataIndex = 0;

    for (let y = 0; y < this.height; y++) {
      // 过滤器字节：0 = 无过滤
      data[dataIndex++] = 0;

      for (let x = 0; x < this.width; x++) {
        // 纯渐变，无任何复杂计算
        const r = Math.floor((x / this.width) * 255);
        const g = Math.floor((y / this.height) * 255);
        const b = 128; // 固定蓝色值
        const a = 255; // 完全不透明

        data[dataIndex++] = r;
        data[dataIndex++] = g;
        data[dataIndex++] = b;
        data[dataIndex++] = a;
      }
    }

    return data;
  }

  // 写入IDAT块（使用zlib压缩）
  async writeIDATChunk(writeStream, data) {
    return new Promise((resolve, reject) => {
      // 使用zlib压缩数据，压缩级别1（最快）
      zlib.deflate(data, { level: 1 }, (err, compressedData) => {
        if (err) {
          reject(err);
          return;
        }

        const chunk = this.createChunk("IDAT", compressedData);
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
    chunk.write(type, 4, 4, "ascii");

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
      console.log("🚀 开始生成简单PNG图片...");
      console.log(`目标尺寸: ${this.width} x ${this.height}`);

      const startTime = Date.now();
      const outputPath = await this.generateIDAT();
      const endTime = Date.now();

      console.log(
        `\n⏱️  生成耗时: ${((endTime - startTime) / 1000).toFixed(2)} 秒`
      );
      console.log(`📁 文件已保存到: ${outputPath}`);

      return outputPath;
    } catch (error) {
      console.error("❌ 生成失败:", error);
      throw error;
    }
  }
}

// 主函数
async function main() {
  try {
    // 生成一个1024x1024的简单图片
    const generator = new SimplePNGGenerator(1024, 1024);
    await generator.generate();
  } catch (error) {
    console.error("生成失败:", error);
    process.exit(1);
  }
}

// 如果直接运行此脚本
if (require.main === module) {
  main();
}

module.exports = SimplePNGGenerator;
