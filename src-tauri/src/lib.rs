// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod render;
mod utils;

use crate::render::image::index::{
    clear_chunk_cache, force_preprocess_chunks, get_image_chunk, get_image_metadata, read_file,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            read_file,
            get_image_metadata,
            get_image_chunk,
            clear_chunk_cache,
            force_preprocess_chunks
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
