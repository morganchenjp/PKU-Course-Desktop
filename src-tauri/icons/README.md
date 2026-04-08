# 应用图标

请将应用图标文件放置在此目录中。

需要的图标文件：

- `32x32.png` - 32x32 像素 PNG
- `128x128.png` - 128x128 像素 PNG
- `128x128@2x.png` - 256x256 像素 PNG (Retina)
- `icon.icns` - macOS 图标集
- `icon.ico` - Windows 图标

可以使用 Tauri 的图标生成工具：

```bash
cargo tauri icon /path/to/source-icon.png
```

源图标建议尺寸：1024x1024 像素
