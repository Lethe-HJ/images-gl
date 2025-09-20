// TODO 下面这个上面的extract_chunk_pixels函数的simd油优化版本 未经测试
// use std::simd::*; // 需要使用 nightly Rust

// fn extract_chunk_pixels_simd(
//     rgba_img: &image::RgbaImage,
//     x: u32,
//     y: u32,
//     width: u32,
//     height: u32,
// ) -> Vec<u8> {
//     let pixel_count = (width * height) as usize;
//     let mut pixels = Vec::with_capacity(pixel_count * 4);
//     let chunk_view = rgba_img.view(x, y, width, height);

//     // 使用 SIMD 一次处理多个像素
//     let simd_width = width / 4 * 4; // 确保能被4整除的部分

//     for y_offset in 0..height {
//         let mut x_offset = 0;

//         // SIMD 处理主循环
//         while x_offset < simd_width {
//             // 一次加载 4 个像素（16字节）
//             // 使用 SIMD 指令并行处理
//             x_offset += 4;
//         }

//         // 处理剩余的像素
//         while x_offset < width {
//             let pixel = chunk_view.get_pixel(x_offset, y_offset);
//             pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], pixel[3]]);
//             x_offset += 1;
//         }
//     }
//     pixels
// }

// 这里可以添加其他工具函数
