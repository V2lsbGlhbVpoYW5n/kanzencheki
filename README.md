# Cheki Gallery

面向地下偶像推活的本地拍立得相册。Tauri 2 + SvelteKit 2 / Svelte 5 + daisyUI 5。

## 启动

```sh
devbox run npm install
devbox run dev       # 浏览器预览 http://127.0.0.1:1420
devbox run desktop   # Tauri 桌面窗口
devbox run check     # Svelte 类型检查与前端构建
devbox run check-rust
```

Devbox 提供 Node 22、Rust 与 Linux GTK / WebKit 依赖。`devbox.lock` 固定解析结果。npm 缓存位于 `.cache/npm`，Cargo 下载和构建分别在 `.cache/cargo`、`.cache/cargo-target`。Devbox 环境内的 XDG 缓存也位于 `.cache`。在首次进入环境之前调用 Devbox，可使用 `XDG_CACHE_HOME="$PWD/.cache" devbox install`；Nix store 仍由系统管理。

## 当前范围

已建立开发环境、桌面外壳、TypeScript 领域模型与照片主导的交互原型。提供网络示例图库、整页连续网格、按需展开的悬浮筛选、悬浮工具条、尺寸调节、搜索与标签筛选、喜欢、待整理、模糊背景的悬浮详情和键盘大图预览；底部胶片条仅切换当前收藏的影像，可添加临时关联影像。JPEG / PNG / WebP 可导入为临时预览，页面关闭后不保留。正式文件入库、SQLite 持久化和完整多版本管理尚未接入。示例来源见 static/demo/SOURCES.md。

设计见 [docs/design.md](docs/design.md)，模型见 [src/lib/model.ts](src/lib/model.ts)。实体拍立得与独立影像通过关联表表达多对多关系，影像与文件版本为一对多。

技术配置依据：[Tauri SvelteKit](https://v2.tauri.app/start/frontend/sveltekit/)、[daisyUI SvelteKit](https://daisyui.com/docs/install/sveltekit/)。
