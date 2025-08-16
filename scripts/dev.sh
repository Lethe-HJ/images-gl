#!/bin/bash

# 开发环境增量编译脚本

echo "🚀 启动 Tauri 开发模式（增量编译）..."

# 检查是否在正确的目录
if [ ! -f "src-tauri/Cargo.toml" ]; then
    echo "❌ 错误：请在项目根目录运行此脚本"
    exit 1
fi

# 设置环境变量优化增量编译
export CARGO_INCREMENTAL=1
export RUSTFLAGS="-C target-cpu=native"

yarn tauri dev

# 清理
echo "🧹 清理进程..."
kill $FRONTEND_PID 2>/dev/null
cd ..

echo "✅ 开发模式已停止"
