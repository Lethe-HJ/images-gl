#!/usr/bin/env node

import { spawn } from 'child_process';
import fs from 'fs';
import path from 'path';

/**
 * 检查ImageMagick是否已安装
 * @returns {Promise<boolean>}
 */
async function checkImageMagick() {
  return new Promise(resolve => {
    const magick = spawn('magick', ['--version']);

    magick.on('error', () => {
      resolve(false);
    });

    magick.on('close', code => {
      resolve(code === 0);
    });

    // 设置超时
    setTimeout(() => {
      magick.kill();
      resolve(false);
    }, 5000);
  });
}

/**
 * 清除当前行并显示进度
 * @param {string} message - 要显示的消息
 */
function updateProgress(message) {
  process.stdout.write(`\r${message}`);
}

/**
 * 使用ImageMagick转换EXR到PNG
 * @param {string} inputPath - 输入的EXR文件路径
 * @param {string} outputPath - 输出的PNG文件路径
 * @param {Object} options - 转换选项
 * @param {number} options.width - 输出宽度（可选）
 * @param {number} options.height - 输出高度（可选）
 * @param {number} options.quality - PNG质量（1-100，默认90）
 * @param {boolean} options.toneMapping - 是否应用色调映射（默认true）
 */
async function convertEXRtoPNGWithImageMagick(
  inputPath,
  outputPath,
  options = {}
) {
  try {
    console.log(
      `🚀 开始转换: ${path.basename(inputPath)} -> ${path.basename(outputPath)}`
    );

    // 检查输入文件是否存在
    if (!fs.existsSync(inputPath)) {
      throw new Error(`输入文件不存在: ${inputPath}`);
    }

    // 检查ImageMagick是否可用
    const hasImageMagick = await checkImageMagick();
    if (!hasImageMagick) {
      throw new Error('ImageMagick未安装。请先安装：brew install imagemagick');
    }

    // 获取输入文件信息
    const stats = fs.statSync(inputPath);
    const fileSizeMB = (stats.size / 1024 / 1024).toFixed(2);
    console.log(`📁 文件大小: ${fileSizeMB} MB`);

    // 构建ImageMagick命令
    const args = [];

    // 输入文件
    args.push(inputPath);

    // 色调映射选项 - 修复全黑问题
    if (options.toneMapping !== false) {
      // 使用更好的色调映射参数
      args.push('-colorspace', 'RGB');
      args.push('-auto-level');
      args.push('-gamma', '2.2');
      args.push('-contrast-stretch', '0.1%');
    }

    // 尺寸调整
    if (options.width || options.height) {
      const resizeArg = [];
      if (options.width) resizeArg.push(options.width);
      if (options.height) resizeArg.push(options.height);
      if (resizeArg.length === 1) resizeArg.push(''); // 保持宽高比
      args.push('-resize', resizeArg.join('x'));
    }

    // 质量设置
    if (options.quality) {
      const quality = Math.max(1, Math.min(100, options.quality));
      args.push('-quality', quality.toString());
    }

    // 输出文件
    args.push(outputPath);

    // 执行转换
    return new Promise((resolve, reject) => {
      const magick = spawn('magick', args);

      const startTime = Date.now();
      const progressBar = '';
      let lastProgressUpdate = 0;

      // 进度反馈 - 单行显示
      const progressInterval = setInterval(() => {
        const elapsed = Date.now() - startTime;
        const elapsedSeconds = (elapsed / 1000).toFixed(1);

        // 检查输出文件是否存在和大小变化
        let fileStatus = '';
        let actualProgress = 0;

        try {
          if (fs.existsSync(outputPath)) {
            const outputStats = fs.statSync(outputPath);
            const outputSizeMB = (outputStats.size / 1024 / 1024).toFixed(2);
            const inputSizeMB = (stats.size / 1024 / 1024).toFixed(2);

            // 基于输出文件大小计算实际进度
            if (outputStats.size > 0) {
              // 假设PNG文件大小约为EXR的30-50%
              const expectedSize = stats.size * 0.4; // 预期40%
              actualProgress = Math.min(
                95,
                Math.floor((outputStats.size / expectedSize) * 100)
              );
              fileStatus = ` | 输出: ${outputSizeMB}MB`;
            }
          }
        } catch (e) {
          // 忽略文件读取错误
        }

        // 基于文件大小和时间的更智能进度计算
        let estimatedProgress;
        const fileSizeMB = stats.size / 1024 / 1024;

        if (fileSizeMB < 50) {
          // 小文件：前80%快速，后20%慢
          if (elapsed < 5000) {
            estimatedProgress = Math.min(80, Math.floor((elapsed / 5000) * 80));
          } else {
            const remainingTime = Math.max(1, (elapsed - 5000) / 1000);
            estimatedProgress = Math.min(
              95,
              80 + Math.floor((remainingTime / 10) * 15)
            );
          }
        } else if (fileSizeMB < 200) {
          // 中等文件：前60%快速，后40%慢
          if (elapsed < 10000) {
            estimatedProgress = Math.min(
              60,
              Math.floor((elapsed / 10000) * 60)
            );
          } else {
            const remainingTime = Math.max(1, (elapsed - 10000) / 1000);
            estimatedProgress = Math.min(
              95,
              60 + Math.floor((remainingTime / 20) * 35)
            );
          }
        } else {
          // 大文件：前40%快速，后60%慢
          if (elapsed < 20000) {
            estimatedProgress = Math.min(
              40,
              Math.floor((elapsed / 20000) * 40)
            );
          } else {
            const remainingTime = Math.max(1, (elapsed - 20000) / 1000);
            estimatedProgress = Math.min(
              95,
              40 + Math.floor((remainingTime / 30) * 55)
            );
          }
        }

        // 优先使用实际进度，如果没有则使用估算进度
        const displayProgress =
          actualProgress > 0 ? actualProgress : estimatedProgress;

        // 创建进度条
        const barLength = 20;
        const filledLength = Math.floor((displayProgress / 100) * barLength);
        const bar =
          '█'.repeat(filledLength) + '░'.repeat(barLength - filledLength);

        // 显示更详细的进度信息
        let progressText = `⏳ 转换中... [${bar}] ${displayProgress}% | 用时: ${elapsedSeconds}s`;

        // 添加文件大小信息
        if (fileSizeMB > 100) {
          progressText += ` | 大文件处理中...`;
        }

        // 添加文件状态信息
        if (fileStatus) {
          progressText += fileStatus;
        }

        // 如果进度卡在95%，显示特殊提示
        if (displayProgress >= 95 && elapsed > 30000) {
          progressText += ` | 即将完成...`;
        }

        // 如果长时间没有进度变化，显示提示
        if (elapsed > 60000 && displayProgress < 50) {
          progressText += ` | 大文件处理中，请耐心等待...`;
        }

        updateProgress(progressText);

        // 每15秒更新一次进度
        if (elapsed - lastProgressUpdate > 15000) {
          lastProgressUpdate = elapsed;
        }
      }, 1000); // 每1秒更新一次

      let stdout = '';
      let stderr = '';

      magick.stdout.on('data', data => {
        stdout += data.toString();
      });

      magick.stderr.on('data', data => {
        stderr += data.toString();
      });

      magick.on('close', code => {
        clearInterval(progressInterval);
        const totalTime = ((Date.now() - startTime) / 1000).toFixed(1);

        // 清除进度条
        process.stdout.write('\n');

        if (code === 0) {
          // 检查输出文件是否存在
          if (fs.existsSync(outputPath)) {
            const outputStats = fs.statSync(outputPath);
            const outputSizeMB = (outputStats.size / 1024 / 1024).toFixed(2);
            const compressionRatio = (
              (outputStats.size / stats.size) *
              100
            ).toFixed(1);

            console.log(
              `✅ 转换完成! 用时: ${totalTime}s | 输出: ${outputSizeMB}MB | 压缩比: ${compressionRatio}%`
            );

            resolve({
              success: true,
              outputPath,
              outputSize: outputStats.size,
              processingTime: totalTime,
              compressionRatio,
            });
          } else {
            reject(new Error('转换完成但输出文件不存在'));
          }
        } else {
          console.error(`❌ 转换失败 (退出码: ${code})`);
          if (stderr) {
            console.error(`错误: ${stderr.trim()}`);
          }
          reject(
            new Error(
              `ImageMagick转换失败，退出码: ${code}\n错误信息: ${stderr}`
            )
          );
        }
      });

      magick.on('error', error => {
        clearInterval(progressInterval);
        process.stdout.write('\n');
        reject(new Error(`启动ImageMagick失败: ${error.message}`));
      });

      // 设置超时
      const timeout = setTimeout(() => {
        clearInterval(progressInterval);
        process.stdout.write('\n');
        magick.kill();
        reject(new Error('转换超时，请检查文件大小和系统资源'));
      }, 600000); // 10分钟超时

      // 清理超时定时器
      magick.on('close', () => {
        clearTimeout(timeout);
      });
    });
  } catch (error) {
    console.error('❌ 转换失败:', error.message);
    return {
      success: false,
      error: error.message,
    };
  }
}

/**
 * 批量转换EXR文件
 * @param {string} inputDir - 输入目录
 * @param {string} outputDir - 输出目录
 * @param {Object} options - 转换选项
 */
async function batchConvertWithImageMagick(inputDir, outputDir, options = {}) {
  try {
    // 确保输出目录存在
    if (!fs.existsSync(outputDir)) {
      fs.mkdirSync(outputDir, { recursive: true });
    }

    // 读取输入目录中的所有文件
    const files = fs.readdirSync(inputDir);
    const exrFiles = files.filter(file => file.toLowerCase().endsWith('.exr'));

    if (exrFiles.length === 0) {
      console.log('在输入目录中没有找到EXR文件');
      return;
    }

    console.log(`📁 找到 ${exrFiles.length} 个EXR文件，开始批量转换...`);

    const results = [];
    for (let i = 0; i < exrFiles.length; i++) {
      const file = exrFiles[i];
      const inputPath = path.join(inputDir, file);
      const outputPath = path.join(outputDir, `${path.parse(file).name}.png`);

      console.log(`\n🔄 [${i + 1}/${exrFiles.length}] 处理: ${file}`);
      const result = await convertEXRtoPNGWithImageMagick(
        inputPath,
        outputPath,
        options
      );
      results.push({ file, ...result });
    }

    // 输出统计信息
    console.log('\n=== 📊 批量转换完成 ===');
    const successCount = results.filter(r => r.success).length;
    const failCount = results.length - successCount;

    console.log(`✅ 成功: ${successCount} 个文件`);
    console.log(`❌ 失败: ${failCount} 个文件`);

    if (failCount > 0) {
      console.log('\n失败的文件:');
      results
        .filter(r => !r.success)
        .forEach(r => {
          console.log(`  - ${r.file}: ${r.error}`);
        });
    }

    // 显示总体统计
    const totalInputSize = results.reduce((sum, r) => {
      if (r.success) {
        const inputPath = path.join(inputDir, r.file);
        try {
          const stats = fs.statSync(inputPath);
          return sum + stats.size;
        } catch (e) {
          return sum;
        }
      }
      return sum;
    }, 0);

    const totalOutputSize = results.reduce((sum, r) => {
      return sum + (r.outputSize || 0);
    }, 0);

    if (totalInputSize > 0) {
      const totalInputMB = (totalInputSize / 1024 / 1024).toFixed(2);
      const totalOutputMB = (totalOutputSize / 1024 / 1024).toFixed(2);
      const overallCompression = (
        (totalOutputSize / totalInputSize) *
        100
      ).toFixed(1);

      console.log(`\n📈 总体统计:`);
      console.log(`   总输入大小: ${totalInputMB} MB`);
      console.log(`   总输出大小: ${totalOutputMB} MB`);
      console.log(`   平均压缩比: ${overallCompression}%`);
    }
  } catch (error) {
    console.error('批量转换失败:', error.message);
  }
}

// 命令行接口
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

if (import.meta.url === `file://${process.argv[1]}`) {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    console.log(`
🎨 EXR到PNG转换工具 (ImageMagick版本)

用法:
  node exr-to-png.js <input> <output> [options]
  node exr-to-png.js --batch <inputDir> <outputDir> [options]

选项:
  --width <number>     输出宽度
  --height <number>    输出高度
  --quality <number>   PNG质量 (1-100, 默认90)
  --no-tone-mapping   禁用色调映射
  --batch             批量转换模式

示例:
  # 单个文件转换
  node exr-to-png.js input.exr output.png --width 1920 --height 1080
  
  # 批量转换
  node exr-to-png.js --batch ./exr-files ./png-files --quality 95

注意:
  - 需要先安装ImageMagick: brew install imagemagick
  - 支持大文件处理
  - 使用专业的图像处理引擎
  - 自动应用HDR到SDR色调映射
        `);
    process.exit(0);
  }

  if (args[0] === '--batch') {
    // 批量转换模式
    if (args.length < 3) {
      console.error('批量转换模式需要指定输入和输出目录');
      process.exit(1);
    }

    const inputDir = args[1];
    const outputDir = args[2];
    const options = parseOptions(args.slice(3));

    batchConvertWithImageMagick(inputDir, outputDir, options);
  } else {
    // 单个文件转换模式
    if (args.length < 2) {
      console.error('需要指定输入和输出文件路径');
      process.exit(1);
    }

    const inputPath = args[0];
    const outputPath = args[1];
    const options = parseOptions(args.slice(2));

    convertEXRtoPNGWithImageMagick(inputPath, outputPath, options);
  }
}

/**
 * 解析命令行选项
 * @param {string[]} args - 命令行参数
 * @returns {Object} 选项对象
 */
function parseOptions(args) {
  const options = {};

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];

    switch (arg) {
      case '--width':
        options.width = parseInt(args[++i]);
        break;
      case '--height':
        options.height = parseInt(args[++i]);
        break;
      case '--quality':
        options.quality = parseInt(args[++i]);
        break;
      case '--no-tone-mapping':
        options.toneMapping = false;
        break;
    }
  }

  return options;
}

export {
  convertEXRtoPNGWithImageMagick,
  batchConvertWithImageMagick,
  checkImageMagick,
};
