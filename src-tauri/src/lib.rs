// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use tauri::ipc::Response;

#[tauri::command]
fn read_file() -> Result<Response, String> {
    // 使用正确的相对路径
    let file_path = "../public/tissue_hires_image.png";

    // 尝试读取文件，处理可能的错误
    match std::fs::read(&file_path) {
        Ok(data) => {
            if data.is_empty() {
                return Err("文件为空".to_string());
            }
            Ok(Response::new(data))
        }
        Err(e) => {
            let error_msg = match e.kind() {
                std::io::ErrorKind::NotFound => format!("文件未找到: {}", file_path),
                std::io::ErrorKind::PermissionDenied => "没有权限读取文件".to_string(),
                std::io::ErrorKind::InvalidInput => "文件路径无效".to_string(),
                _ => format!("读取文件时发生错误: {}", e),
            };
            Err(error_msg)
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![read_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
