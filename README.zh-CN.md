# KanzenCheki

[English](README.md) · [简体中文](README.zh-CN.md)

KanzenCheki 是一款本地优先的桌面软件，用来整理拍立得收藏、每张收藏的多份影像，以及相关的人物和回忆。技术栈为 Tauri 2、Rust、SQLite、SvelteKit 和 Svelte 5。

名字 neta 了 *完全聖域* コール。

> **项目状态**：目前为 0.1.0 第一版的 review 阶段，正在检查功能与数据处理。预发布安装包通过 GitHub Actions 在版本标签上构建并发布。

## 功能

- **以实体收藏为中心**：将同一张拍立得的多份扫描图或照片归在一起，记录日期、人物、团体、活动、标签、备注与喜欢状态。
- **本地存储**：元数据保存在 SQLite，原件以普通文件存储。可以复制到软件管理的图库，也可以管理已配置外置目录中的原件；目录离线时仍可浏览已有的本机缓存。
- **整理和查找**：搜索文字、人物与标签，筛选喜欢和待整理收藏，排序、批量编辑与归并。
- **影像处理**：指定封面、裁切、透视校正、旋转与全尺寸查看。裁切保留原件；旋转会修改原件文件。
- **人物与备份**：管理人物资料、附件和 Markdown 文章；支持完整或增量备份，并在恢复前预览文件冲突。
- **离线建议**：在本机检测人脸并建议拍摄类型和人物，所有建议都需要核验。
- **界面语言**：简体中文、English、日本語。

桌面版会持久保存图库；浏览器预览只使用临时示例数据，刷新后重置。统计路由已经建立，但页面尚未实现。

> [!CAUTION]
> 这个项目是 Vibe Coding First 的项目，且目前代码没有经过完整的人工审阅，可能存在数据风险，请经常使用备份功能，并尽量进行手动备份来降低数据丢失或损坏的风险。
>
> 注意这个项目目前还在 Pre-Release 阶段，可能存在破坏性的变更。

## 快速开始

### 环境要求

[Devbox](https://www.jetify.com/devbox) 提供本项目使用的 Node.js 22、Rust，以及 Linux GTK/WebKit 依赖。目前已在 Linux 上验证原生桌面构建，其他平台仍需验证。

~~~sh
devbox run npm ci
devbox run desktop
~~~

桌面开发窗口会在 1420 端口启动 Vite 服务。若只想查看临时浏览器示例，可以运行：

~~~sh
devbox run npm run demo:images          # 在本机下载第三方示例图片
devbox run dev
~~~

### 桌面安装包

推送 `v0.1.0` 等版本标签后，GitHub Actions 会在全部安装包构建成功时发布 [GitHub 预发布版本](https://github.com/V2lsbGlhbVpoYW5n/kanzencheki/releases)。其中包含 Windows x64 的 NSIS 安装程序、适用于 Intel 和 Apple Silicon 的 macOS DMG，以及 Linux x64 的 AppImage。Pull Request 和推送到 `main` 会运行前端与 Rust 检查。

macOS 安装包采用临时签名，尚未公证，首次打开时可能需要手动批准。Windows 安装程序尚未签名，可能触发 SmartScreen 提示。请从本项目的 GitHub Release 页面下载安装包。

### 构建与验证

~~~sh
devbox run check                         # Svelte 检查和前端构建
devbox run npm test                      # 前端测试
devbox run cargo test --manifest-path src-tauri/Cargo.toml --lib
devbox run npm run desktop:build         # 构建桌面可执行文件，不生成安装包
devbox run desktop-release               # 运行已构建的程序
~~~

Linux 可执行文件位于 .cache/cargo-target/release/kanzencheki。目前构建流程尚不生成安装包。

## 图库与数据

桌面版默认在系统应用数据目录中打开图库。Linux 上通常为：

~~~text
~/.local/share/app.kanzencheki.desktop/library/
~~~

更名后仍保留原应用标识，使已有图库继续可被发现。启动前设置 CHEKI_LIBRARY_DIR 可以改用其他图库目录；这个环境变量也因兼容性而保留。

图库包含 SQLite 数据库、托管原件、浏览缓存和导入暂存文件。导入到本机图库的原件会被复制，不修改来源文件；已配置外置目录中的原件可能被软件原地重命名或旋转。备份时需要同时保存数据库和原件。建议使用内置备份功能获取一致的快照，不要只复制运行中的 SQLite 主文件。

支持导入 JPEG、PNG、WebP 和 TIFF。部分浏览图处理使用 ImageMagick，缺失时会尝试 Rust 解码器。桌面版从空图库开始；示例图片只用于浏览器演示。

## 项目结构

| 路径 | 内容 |
| --- | --- |
| src/ | SvelteKit 界面、翻译和浏览器示例 |
| src-tauri/ | Rust 桌面后端、存储和影像处理 |
| src-tauri/models/ | 本机模型说明与第三方许可证 |
| static/demo/ | 浏览器示例图片来源；按需下载到本机 |
| docs/ | 保留作历史参考的已弃用设计文档 |

## Roadmap

- [x] CI/CD：自动检查、构建与发布
- [ ] 移动端
- [ ] 局域网同步
- [ ] 统计页面

这些是下一步方向，不代表已确定的发布时间。

## 贡献

欢迎在 GitHub Issues 提交 bug 或新功能建议。报告 bug 时，请尽量附上复现步骤和运行环境。这个软件主要围绕个人需求开发；项目会长期维护，但新功能会按实际需要选择，不一定会激进地加入功能。

## 许可证

项目源代码采用 [MIT License](LICENSE)。随附的人脸模型适用各自的许可证，详见 [src-tauri/models/](src-tauri/models/README.md)。浏览器示例图片为第三方素材，不再纳入 Git 仓库；来源见[图片说明](static/demo/SOURCES.md)。MIT 许可证不授予这些图片或模型的使用权。下载脚本会校验每张图片的 SHA-256，若原站内容变动则停止使用。
