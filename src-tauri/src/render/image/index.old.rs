use crate::utils::time::get_time;
use image::GenericImageView;
use memmap2::{MmapMut, MmapOptions};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::ipc::Response;
use tauri::ipc::Response;
use tauri::State;
use tauri::State;

#[tauri::command]
pub fn read_file() -> Result<Response, String> {
    let start_time = get_time();
    println!("[RUST] 开始读取图片: {}ms", start_time);

    // 使用正确的相对路径
    let file_path = "../public/tissue_hires_image.png";

    // 图片解码优化：跳过格式检测，直接使用 PNG 解码器
    let decode_start = get_time();

    // 直接使用 PNG 解码器，跳过格式检测
    let file = std::fs::File::open(file_path).map_err(|e| format!("文件打开失败: {}", e))?;
    let reader = std::io::BufReader::new(file);

    // 使用 PNG 解码器，避免格式检测开销
    let decoder = image::codecs::png::PngDecoder::new(reader)
        .map_err(|e| format!("PNG解码器创建失败: {}", e))?;

    let img =
        image::DynamicImage::from_decoder(decoder).map_err(|e| format!("PNG解码失败: {}", e))?;

    let decode_end = get_time();

    println!(
        "[RUST] PNG直接解码完成: {}ms (耗时: {}ms)",
        decode_end,
        decode_end - decode_start
    );

    // 获取图片尺寸并打印（用于调试）
    let (width, height) = img.dimensions();
    println!("[RUST] 图片尺寸: {}x{}", width, height);

    // RGBA转换优化：直接获取像素数据，避免不必要的转换
    let convert_start = get_time();

    // 检查图片是否已经是 RGBA8 格式，避免不必要的转换
    let pixels = match img {
        image::DynamicImage::ImageRgba8(rgba) => {
            println!("[RUST] 图片已经是 RGBA8 格式，直接使用");
            rgba.into_raw()
        }
        _ => {
            println!("[RUST] 图片需要转换为 RGBA8 格式");
            let rgba_img = img.to_rgba8();
            rgba_img.into_raw()
        }
    };

    let convert_end = get_time();
    println!(
        "[RUST] RGBA处理完成: {}ms (耗时: {}ms)",
        convert_end,
        convert_end - convert_start
    );

    // 创建包含尺寸信息的头部（8字节）
    let mut data = Vec::with_capacity(8 + pixels.len());
    data.extend_from_slice(&width.to_be_bytes()); // 4字节宽度
    data.extend_from_slice(&height.to_be_bytes()); // 4字节高度

    data.extend_from_slice(&pixels); // 像素数据

    let end_time = get_time();
    let data_size = data.len(); // 在移动前保存大小

    // 返回带有尺寸信息的像素数据
    let response = Ok(Response::new(data));
    println!(
        "[RUST] 图片处理完成: {}ms (总耗时: {}ms)",
        end_time,
        end_time - start_time
    ); // 总体耗时最少记录为 2190ms
    println!(
        "[RUST] 数据大小: {} 字节 (头部: 8字节, 像素: {}字节)",
        data_size,
        pixels.len()
    );
    return response;
    // 这里的返回是直接将内存区域传递给前端进行读取, 不需要进行任何序列化和反序列化, 甚至不需要拷贝
    // 经测试 这里返回数据与 ts 读取到数据耗时约 67ms
}
