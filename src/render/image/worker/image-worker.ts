// 图片处理 Worker - TypeScript 版本

// 定义消息数据类型
interface WorkerMessage {
  imageBuffer: ArrayBuffer;
  width?: number;
  height?: number;
}

interface WorkerResponse {
  success: boolean;
  imageBitmap?: ImageBitmap;
  width?: number;
  height?: number;
  error?: string;
}

self.onmessage = async function (e: MessageEvent<WorkerMessage>) {
  const startTime = Date.now();
  console.log(`[WORKER] 开始处理图片: ${startTime}ms`);

  const { imageBuffer } = e.data;

  try {
    // 解析图片数据
    const parseStartTime = Date.now();
    const data = new Uint8Array(imageBuffer);
    const dataView = new DataView(data.buffer);
    const imgWidth = dataView.getUint32(0);
    const imgHeight = dataView.getUint32(4);
    const pixels = new Uint8Array(data.slice(8));
    const parseEndTime = Date.now();
    console.log(
      `[WORKER] 数据解析完成: ${parseEndTime}ms (耗时: ${
        parseEndTime - parseStartTime
      }ms)`
    );

    // 创建 ImageData
    const imageDataStartTime = Date.now();
    const imageData = new ImageData(
      new Uint8ClampedArray(pixels),
      imgWidth,
      imgHeight
    );
    const imageDataEndTime = Date.now();
    console.log(
      `[WORKER] ImageData创建完成: ${imageDataEndTime}ms (耗时: ${
        imageDataEndTime - imageDataStartTime
      }ms)`
    );

    // 创建 ImageBitmap（在 Worker 中处理）
    const bitmapStartTime = Date.now();
    const imageBitmap = await createImageBitmap(imageData);
    const bitmapEndTime = Date.now();
    console.log(
      `[WORKER] ImageBitmap创建完成: ${bitmapEndTime}ms (耗时: ${
        bitmapEndTime - bitmapStartTime
      }ms)`
    );

    const totalEndTime = Date.now();
    console.log(
      `[WORKER] 处理完成: ${totalEndTime}ms (总耗时: ${
        totalEndTime - startTime
      }ms)`
    );

    // 返回处理结果
    const response: WorkerResponse = {
      success: true,
      imageBitmap: imageBitmap,
      width: imgWidth,
      height: imgHeight,
    };

    self.postMessage(response, { transfer: [imageBitmap] }); // 转移 ImageBitmap 所有权
  } catch (error) {
    const errorTime = Date.now();
    const errorMessage = error instanceof Error ? error.message : "未知错误";
    console.error(
      `[WORKER] 处理失败: ${errorTime}ms (耗时: ${errorTime - startTime}ms)`,
      error
    );

    const errorResponse: WorkerResponse = {
      success: false,
      error: errorMessage,
    };

    self.postMessage(errorResponse);
  }
};
