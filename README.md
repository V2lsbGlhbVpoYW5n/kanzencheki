# Cheki Gallery

面向地下偶像推活的本地拍立得相册。Tauri 2 + Rust / SQLite + SvelteKit / Svelte 5 + daisyUI。

## 启动与验证

```sh
devbox run npm install
devbox run desktop               # Tauri 开发窗口，真实本地图库
devbox run desktop-release       # 启动编译好的独立桌面程序
devbox run dev                   # 浏览器示例 http://127.0.0.1:1420，不持久化
devbox run npm run desktop:build # 编译独立桌面可执行文件（暂不打安装包）
devbox run check                 # Svelte 类型检查与前端构建
devbox run npm test              # 标签搜索与元数据交互规则
devbox run cargo test --manifest-path src-tauri/Cargo.toml --lib
```

`desktop` 会启动端口 1420 的开发服务器，如已运行浏览器预览需先停止它。Linux 可执行文件位于 `.cache/cargo-target/release/cheki-gallery`，可通过 `devbox run desktop-release` 启动，独立构建不依赖 Vite 服务。其他系统的原生构建仍需在对应系统验证。

Devbox 提供 Node 22、Rust 与 Linux GTK / WebKit 依赖，`devbox.lock` 固定解析结果。npm 缓存位于 `.cache/npm`，Cargo 下载与构建分别位于 `.cache/cargo`、`.cache/cargo-target`。首次调用 Devbox 可用 `XDG_CACHE_HOME="$PWD/.cache" devbox install`；Nix store 仍由系统管理。

## 本地图库

默认使用系统应用数据目录下 `app.chekigallery.desktop/library`，Linux 通常为 `~/.local/share/app.chekigallery.desktop/library`。当前可通过环境变量 `CHEKI_LIBRARY_DIR` 指定另一图库目录，设置页面尚未提供路径选择。

```text
library/
  library.sqlite      # 收藏信息、人物、标签与影像关系
  library.sqlite-wal  # SQLite 运行时日志（可能存在）
  library.sqlite-shm
  library.lock        # 防止多个进程同时管理同一图库
  originals/          # 按原始字节复制的照片，不转码、不修改来源文件
  previews/           # 最长边 1800px 的 JPEG 浏览图
  staging/            # 导入临时文件与可恢复的导入日志
```

文件名例如 `2026-08-27_小明+小蓝等5人_a38f9e21b0.tiff`。日期或人物尚未补全时使用 `未定日期` / `未定人物`；保存资料后更新托管原件文件名。数据库保留导入前文件名。备份应在退出程序后复制整个图库目录，勿仅复制运行中的 SQLite 主文件。

- **Cheki** 是实体收藏，保存日期、多人列表、活动、自由文字标签、拍摄类型、备注和独立的 `favorite` 布尔值。
- **Asset** 是一次扫描或拍摄。Cheki ↔ Asset 是多对多，可在胶片带添加新影像或关联已有影像；共享影像只存一份，文件名跟随最初关联的收藏。
- **Rendition** 是同一影像的文件表示，目前生成 `original` 和 `display`。界面加载浏览图，原始 TIFF 不传入前端内存。
- 人物与标签均通过实体表和关联表存储。多人切 / 团切保存全部人物，文件名只取前两名及总人数，不把人物列表当作一段不可查询的备注。
- 原生文件选择支持 JPEG、PNG、WebP、TIFF 和批量导入。导入使用暂存副本及持久化日志，启动时恢复未完成入库；元数据保存和文件改名意图一起提交，重启可继续改名。
- 预览解码预算为 512 MiB。超大或特殊 TIFF 可能无法生成预览，但仍保留原件、尺寸、大小和失败说明，可重试；尚未实现针对超大 TIFF 的分块解码。

## 相册交互

整页连续相册、顶部浮动导航（统计 / 相册 / 人物），统计与人物页留空。底部爱心和 Inbox 按钮切换当前视图，再次点击返回全部收藏；左上角显示当前视图和结果数量。

Inbox 条件为缺少日期或人物。进入时固定这一批收藏，补全并保存后保留卡片并覆盖半透明完成提示，直到离开 Inbox 后再进入才重新筛选。活动、标签、类型和备注不影响是否完成。

大图旁的浮动信息卡可编辑元数据，底部显示当前影像的文件名、分辨率、大小。人物和标签支持输入、候选补全、Enter 添加及删除。搜索 `#夏日 #舞台` 要求两个标签同时满足；含空格标签用 `#"夏日 演出"`，补全会自动加引号。普通文字搜索人物、活动、日期、备注、类型、标签及原文件名。`Ctrl/Cmd + Enter` 保存，左右方向键切换当前收藏影像，关闭未保存编辑会提示。

浏览器版本仍使用内存示例数据，刷新后重置；正式桌面版从空图库开始，不自动导入示例图片。示例来源见 `static/demo/SOURCES.md`。

当前未实现：文件夹监听、编辑与裁切导出、同步、识别、人物及统计页面、设置 UI、安装包发布。已有目录不做静默自动扫描，照片通过导入入口纳入管理。

## Linux / Wayland 图形环境

在本机 EndeavourOS + Wayland 中，直接启动 Nix 构建的 WebKitGTK 会出现 `EGL_BAD_PARAMETER`。`scripts/desktop.sh` 使用 Devbox 锁定的 nixGLIntel 衔接 Mesa/EGL 驱动；Wayland 会话下默认使用原生 Wayland，并尊重已有的 `GDK_BACKEND` 设置。没有关闭 WebKit 合成或模糊效果，也不依赖 XWayland。已验证窗口正常显示。

这里的 nixGLIntel 是 Mesa 包装器，也适用于 AMD Mesa 驱动；专有 NVIDIA 驱动环境尚未验证，需要匹配内核驱动版本的 nixGLNvidia。说明见 [nixGL](https://github.com/nix-community/nixgl)。包装器只作用于本项目启动命令，不修改系统图形配置。
