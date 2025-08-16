# WebGL GLSL Editor

到 vscode 中搜索到这个插件后 进入这个插件的页面 然后点击齿轮图标 然后点击'download Vsix'
将下载下来的 vsix 文件拖动到 cursor 的 extension 的侧边视图的空白处即可安装

# glsl lint

安装插件 glsl lint

brew install glslang
glslangValidator --version
file $(which glslangValidator)
复制其路径
粘贴到 编辑器配置的 glslang validator path 中
