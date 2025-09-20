use serde::{Deserialize, Serialize};

// Chunk 元数据结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChunkInfo {
    pub x: u32,       // chunk 在图片中的 X 坐标
    pub y: u32,       // chunk 在图片中的 Y 坐标
    pub width: u32,   // chunk 宽度
    pub height: u32,  // chunk 高度
    pub chunk_x: u32, // chunk 的 X 索引
    pub chunk_y: u32, // chunk 的 Y 索引
}

// 图片元数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub total_width: u32,       // 图片总宽度
    pub total_height: u32,      // 图片总高度
    pub chunk_size_x: u32,      // chunk 大小 X 方向（正方形）
    pub chunk_size_y: u32,      // chunk 大小（正方形）
    pub col_count: u32,         // X 方向的 chunk 数量
    pub row_count: u32,         // Y 方向的 chunk 数量
    pub chunks: Vec<ChunkInfo>, // 所有 chunk 信息
}
