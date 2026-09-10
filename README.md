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

默认使用系统应用数据目录下 `app.chekigallery.desktop/library`，Linux 通常为 `~/.local/share/app.chekigallery.desktop/library`。当前可通过环境变量 `CHEKI_LIBRARY_DIR` 指定另一图库目录，右下方的设置按钮打开浮层，可添加多个外部原件目录、重新定位离线目录和重试缓存。当前本机数据库／缓存目录仍通过此环境变量选择，尚未提供原地迁移。

```text
library/
  library.sqlite      # 收藏信息、人物、标签与影像关系
  library.sqlite-wal  # SQLite 运行时日志（可能存在）
  library.sqlite-shm
  library.lock        # 防止多个进程同时管理同一图库
  originals/          # 按原始字节复制的照片，不转码、不修改来源文件
  previews/           # 最长边 1800px 的完整缓存与裁切浏览图
  staging/            # 导入临时文件与可恢复的导入日志
```

文件名例如 `2026-08-27_小明+小蓝等5人_a38f9e21b0.tiff`。日期或人物尚未补全时使用 `未定日期` / `未定人物`；保存资料后更新托管原件文件名。数据库保留导入前文件名。备份应在退出程序后复制整个图库目录，勿仅复制运行中的 SQLite 主文件。

- **Cheki** 是实体收藏，保存日期、多人列表（团切改用独立团体名）、活动、自由文字标签、拍摄类型、备注和独立的 `favorite` 布尔值。
- **Asset** 是一次扫描或拍摄。Cheki ↔ Asset 是多对多，可在胶片带添加新影像或关联已有影像；共享影像只存一份，文件名跟随最初关联的收藏。
- **Rendition** 是同一影像的文件表示，支持原件 `original`、手动添加的同源文件 `version:<id>`、完整本机缓存 `base` 和裁切浏览图 `display`。界面加载浏览图，原始 TIFF 不传入前端内存。
- 人物与标签均通过实体表和关联表存储。多人切保存全部人物；团切单独保存团体名，并清除人物关联，文件名只取前两名及总人数，不把人物列表当作一段不可查询的备注。
- 原生文件选择支持 JPEG、PNG、WebP、TIFF 和批量导入。导入使用暂存副本及持久化日志，启动时恢复未完成入库；元数据保存和文件改名意图一起提交，重启可继续改名。
- Devbox 提供 ImageMagick，生成最长边 1800px 的 JPEG，支持 EXIF 自动方向。处理限制为内存 256 MiB、映射 512 MiB、磁盘缓存 8 GiB、2 线程、300 秒；缺少 ImageMagick 时回退到有 512 MiB 解码预算的 Rust 解码器。特殊或超大图像仍可能处理失败，原件与待处理任务保留，可重试。
- 外部原件使用目录 ID + 相对路径登记，不移动、不改名。本机缓存保持可用；修改收藏资料和已有缓存的裁切不依赖原件。应用在相册空闲时每 30 秒检查连接状态，检测到待处理原件重新可用后重试缓存，也可在设置里刷新或手动重试。

## 相册交互

整页连续相册、顶部浮动导航（统计 / 相册 / 人物），统计与人物页留空。底部爱心和 Inbox 按钮切换当前视图，再次点击返回全部收藏；左上角显示当前视图和结果数量。

Inbox 条件为缺少日期或人物；团切改为缺少日期或团体名。进入时固定这一批收藏，补全并保存后保留卡片并覆盖半透明完成提示，直到离开 Inbox 后再进入才重新筛选。活动、标签、类型和备注不影响是否完成。

大图右侧分为“收藏信息”和“影像版本”两个页签，只保留右上关闭入口。日期使用 Cally + daisyUI 日历，菜单统一为玻璃浮层。信息卡可编辑元数据，底部显示当前影像的文件名、分辨率、大小。人物和标签支持输入、候选补全、Enter 添加及删除。搜索 `#夏日 #舞台` 要求两个标签同时满足；含空格标签用 `#"夏日 演出"`，补全会自动加引号。普通文字搜索人物、团体、活动、日期、备注、类型、标签及原文件名。`Ctrl/Cmd + Enter` 保存，左右方向键切换当前收藏影像，关闭未保存编辑会提示。

浏览器版本仍使用内存示例数据，刷新后重置；正式桌面版从空图库开始，不自动导入示例图片。示例来源见 `static/demo/SOURCES.md`。

## 影像与文件版本

导入时可选择：每个文件独立建收藏、全部属于同一收藏的不同影像、全部属于同一影像的文件版本。原件可复制到本机图库，或引用已配置目录内的文件。设置中的“扫描目录并登记照片”递归扫描，跳过符号链接和已登记文件。

自动封面优先有本机预览的影像，再按像素尺寸、无损格式和来源进行启发式选择；可手动指定影像封面，或指定同一影像的某个文件版本用于生成浏览图。场景返切暂不纳入导入分类，后续在人物下单独管理。

影像页签提供文件名、尺寸、大小、所在目录及在线状态；可添加同源版本、关联已有影像、将另一张收藏归并过来，或将当前影像拆成独立收藏。归并仅合并影像关系，来源收藏及其资料保留在回收站，可恢复。相似候选依据预览差分哈希和文件名启发式生成，只作人工归并提示，不能保证识别同一实体。

裁切采用归一化矩形参数，支持拖动位置／右下角调整大小和 Instax Mini / Square / Wide 的正反方向比例。自动裁切为扫描底色对比的边界建议，需要检查并确认；取消裁切恢复完整缓存。当前不做透视校正、旋转矫正、一张扫描中自动分割多张拍立得或全分辨率裁切导出。

## 回收站与任务中心

底部“选择收藏”进入多选，支持 Shift 连选、全选当前结果与 Ctrl/Cmd+A。批量删除第一次点击进入确认，第二次点击才移入回收站；改变选择会清除确认状态。回收站可以单独或批量恢复，当前不提供彻底删除和清空，文件不会被删除。

操作完成消息显示为短暂悬浮提示，左下角圆形按钮展开任务与消息中心。批量导入按已处理文件数报告进度，单文件解码阶段显示处理中，不伪造解码百分比。缓存待处理项持久化在 SQLite 中，消息列表为本次运行的会话记录。当前不提供任务取消或字节级拷贝进度。

尚未实现：目录文件变动的自动入库、同步、人物识别、场景返切管理、人物及统计页面、安装包发布。

## Linux / Wayland 图形环境

在本机 EndeavourOS + Wayland 中，直接启动 Nix 构建的 WebKitGTK 会出现 `EGL_BAD_PARAMETER`。`scripts/desktop.sh` 使用 Devbox 锁定的 nixGLIntel 衔接 Mesa/EGL 驱动；Wayland 会话下默认使用原生 Wayland，并尊重已有的 `GDK_BACKEND` 设置。没有关闭 WebKit 合成或模糊效果，也不依赖 XWayland。已验证窗口正常显示。

这里的 nixGLIntel 是 Mesa 包装器，也适用于 AMD Mesa 驱动；专有 NVIDIA 驱动环境尚未验证，需要匹配内核驱动版本的 nixGLNvidia。说明见 [nixGL](https://github.com/nix-community/nixgl)。包装器只作用于本项目启动命令，不修改系统图形配置。

## 大文件回归

已用 19,000 × 19,000、1,083,009,768 字节的未压缩 TIFF 验证完整导入及 1800px 离线缓存生成。可选压力测试会创建并自动清理临时文件，需数 GB 空闲磁盘：

```sh
devbox run bash -c 'cargo test --manifest-path src-tauri/Cargo.toml --lib gigabyte_tiff_generates_offline_cache -- --ignored --nocapture --test-threads=1'
```
