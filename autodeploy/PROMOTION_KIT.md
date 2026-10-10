# 🎯 VidNest 全渠道推广实战分发物料与执行手册 (Promotion Kit)

> **定位**: 本地 AI 视频智能中枢与下载工坊 (Local-First Video Intelligence & Downloader)  
> **官网地址**: [https://novamindlab.github.io/VidNest/](https://novamindlab.github.io/VidNest/)  
> **代码仓库**: [https://github.com/NovaMindLab/VidNest](https://github.com/NovaMindLab/VidNest)  
> **最新版本**: `v1.1.0` (跨平台 Windows EXE / macOS DMG / Linux AppImage)

---

## 一、 已自动执行完成的推广动作 (Automated Execution Completed)

1. ✅ **GitHub Pages 静态官网全渠道矩阵更新上线**:
   - `https://novamindlab.github.io/VidNest/` (官方营销主站，内置系统探测直达下载)
   - `https://novamindlab.github.io/VidNest/download.html` (全平台独立多架构下载中心，提供真实 SHA-256 校验码)
   - `https://novamindlab.github.io/VidNest/share.html` (移动端扫码防拦截分流页，微信/QQ智能提示+1.8s看门狗唤起)
   - `https://novamindlab.github.io/VidNest/vs-eagle.html` (针对 Eagle 素材管理用户的对比转换落地页)
   - `https://novamindlab.github.io/VidNest/vs-capcut-jianying.html` (针对剪映/CapCut 用户的素材库对比落地页)
   - `https://novamindlab.github.io/VidNest/vs-jellyfin-plex.html` (针对 NAS/家庭影院用户的轻量化桌面端落地页)
   - `https://novamindlab.github.io/VidNest/vs-local-video-manager.html` (针对文件夹管理痛点的落地页)
   - `https://novamindlab.github.io/VidNest/sitemap.xml` & `robots.txt`

2. ✅ **IndexNow 搜索引擎秒级推流 (实时推送已生效)**:
   - 已调用 `api.indexnow.org` 官方推流网关，向 **Microsoft Bing、Seznam、Yandex** 提交了全部 8 个目标落地页 URL。
   - 响应状态码：`HTTP 202 Accepted`，搜索引擎爬虫将在数分钟内加速收录与展现。

3. ✅ **国际主流包管理器清单自动生成 (带真实 v1.1.0 哈希)**:
   - **WinGet**: `autodeploy/manifests/winget/NovaMindLab/VidNest/1.1.0/` 三文件 YAML 已就绪（带 `/S` 静默安装参数与 SHA-256：`A876256C1328D2393426EF6C1B0F926599C85D31E4BDE4385AC32F9112A88857`）。
   - **Scoop**: `autodeploy/manifests/scoop/vidnest.json` 已就绪。
   - **Homebrew Cask**: `autodeploy/manifests/homebrew/vidnest.rb` 已就绪（带隔离属性移除 hook，解决 macOS Sequoia 报已损坏问题）。

---

## 二、 核心渠道即时复制发布文案包 (Ready-to-Post Copy Kit)

### 1. 【V2EX】技术极客社区
* **推荐节点**: `分享创造` 或 `开源软件`
* **标题**: 用 Tauri 2 + Rust 写了个完全本地的视频智能管理与下载神器：万部视频毫秒检索，零云端隐私泄露
* **正文**:
```markdown
各位 V 友大家好！

作为视频素材重度用户，日常收集了几百 G 的剪辑素材和技术网课。之前用普通文件夹找一段视频往往翻十几层子目录；用网盘不仅限速还有被和谐封号风险；用商业素材软件导入大量 4K 长视频后界面经常卡死；用纯 yt-dlp 命令行又老遇到 403 权限拦截……

于是花时间用 Tauri 2 + Rust + Vue 3 打造了这款真正 Local-First 的本地视频管理中枢 —— **VidNest (Local Video Intelligence)**。

🔥 核心功能亮点：
1. **原地零拷贝毫秒检索**：基于 SQLite + FTS5 全文索引，不改变你的原硬盘目录结构，视频文件名、编码、时长、分辨率、标签组合秒级过滤呈现。
2. **嗅探下载中心**：内置独立托管的 yt-dlp 引擎，自动规避 403 防盗链，多分辨率音视频提取与混流，下载后一键直达本地库。
3. **极致轻量原生**：Rust 原生内核，内存占用通常低于 80MB，冷启动毫秒级呈现，支持独立画中画弹窗播放。
4. **全平台与暗夜体验**：支持 macOS (Apple Silicon M系列 / Intel)、Windows 10/11、Linux，全量深浅色模式与中英国际化。

100% 自由开源，绝不上传任何私人数据。

🌐 官网体验：https://novamindlab.github.io/VidNest/
💻 GitHub 仓库：https://github.com/NovaMindLab/VidNest
🪟 Windows / 🍎 macOS / 🐧 Linux 安装包已在 Release 全量发布，欢迎试用与提反馈，求个 Star ⭐️ 支持！
```

---

### 2. 【少数派 (sspai) / 知乎】效率工具评测
* **推荐话题/专栏**: 视频剪辑效率神器、优质桌面工具推荐、如何管理电脑里的数万个视频
* **标题**: 告别混乱文件夹与卡死！开源本地视频管理神器 VidNest：毫秒级检索 + 智能下载
* **正文结构**:
  - **现状痛点**: 传统文件夹只能单层树状归档，找素材极其痛苦；云盘限速且隐私堪忧；
  - **功能对比**:
    - 对比 Eagle：VidNest 免费开源，针对海量视频流深度优化，不破坏原有目录；
    - 对比剪映素材库：VidNest 解决剪映工程文件散乱、吃满系统盘、无全局检索的问题；
  - **特色演示**: 介绍智能网格流、独立画中画窗口、支持 YouTube/TED/Vimeo 等多站点的嗅探下载器；
  - **安装指引**: 附上官网 `https://novamindlab.github.io/VidNest/`。

---

### 3. 【小红书 / 微信公众号】视觉向种草
* **小红书标题**: 剪辑师/网课党狂喜！挖到一个神仙本地视频管理工具😭
* **封面图建议**: VidNest 暗夜模式下的媒体库卡片截图、独立播放器悬浮窗截图
* **正文**:
```text
作为一个囤了上千个视频素材和教程的深度用户，以前找素材真的要疯：
❌ 文件夹套文件夹，根本记不住放哪了
❌ 很多收费软件动不动就上百块，存几个 4K 视频预览还卡死
❌ 网上的参考片源老是下载失败报错

最近挖到这个完全免费、界面高级感拉满的神器 —— VidNest！
✨ 毫秒级搜索：打字秒出结果，不用把视频搬来搬去，完全不占双倍空间！
✨ 内置下载器：复制网页视频链接直接下载 1080P/4K，下载完自动进视频库！
✨ 轻量流畅：苹果电脑和 Windows 都能用，黑夜模式质感拉满，完全免费！

在 GitHub 搜「VidNest」或者去官网体验～
#视频剪辑 #剪辑素材 #生产力工具 #效率软件 #Mac必备软件 #Windows软件 #学生党必备 #网课
```

---

### 4. 【Product Hunt / Show HN / Reddit】海外出海首发
* **Tagline**: Local-first video intelligence & downloader built with Tauri 2 and Rust
* **Show HN Title**: Show HN: VidNest – Local-first video library and downloader built with Tauri 2 & Rust
* **Post Content**:
```markdown
Hey everyone!

I built VidNest, an open-source, local-first desktop application designed for video creators, researchers, and multimedia collectors.

Why build another video manager?
1. Existing tools like Eagle or design managers struggle with gigabytes of 4K video files and cost money.
2. Cloud drives are slow and raise privacy concerns.
3. Command-line yt-dlp is powerful but difficult for daily creators to organize downloaded assets seamlessly.

Features:
- Sub-millisecond Search: Powered by SQLite FTS5, instantly filter by codecs, resolution, date, and custom tags.
- Built-in Downloader: Managed yt-dlp engine with anti-403 bypass and multi-format audio/video extraction.
- Ultra Lightweight: Built with Tauri 2 + Rust, consuming <80MB RAM with native dark mode & full i18n.
- 100% Local & Private: No telemetry, no account required, completely free & open source under MIT.

Check it out:
• Website: https://novamindlab.github.io/VidNest/
• GitHub: https://github.com/NovaMindLab/VidNest

Would love your thoughts, feedback, and feature requests!
```

---

## 三、 包管理器提交一键指引 (Package Manager Distribution)

### 1. 提交至 Homebrew (macOS)
在已有的个人 Tap 仓库（如 `homebrew-tap`）中添加：
```bash
# 复制已生成的公式
cp autodeploy/manifests/homebrew/vidnest.rb /path/to/homebrew-tap/Casks/
# 提交并推送后，用户即可执行：
brew install --cask your-username/tap/vidnest
```

### 2. 提交至 Scoop (Windows)
将 `autodeploy/manifests/scoop/vidnest.json` 提交到 Scoop bucket，用户只需执行：
```powershell
scoop install vidnest.json
```

### 3. 提交至微软 WinGet 官方源
使用微软官方 `wingetcreate` 工具提交（已包含结构化清单）：
```powershell
wingetcreate submit autodeploy/manifests/winget/NovaMindLab/VidNest/1.1.0/
```
