# Changelog

Release notes here become both the GitHub release body and the "what's new"
text shown by the in-app updater.

## [0.1.0] - 2026-10-26

首个可分发版本。

### 浏览
- 我的云盘（懒加载目录树）、最近、星标、与我共享、回收站
- 面包屑与前进/后退/上一级，当前列表筛选，Enter 全盘搜索
- 按名称/大小/修改时间/类型排序（文件夹优先），虚拟滚动
- 多选（单击 / ⌘ / Shift）、右键菜单、拖拽文件上传

### 预览与详情
- 文本、图片（含 SVG）、PDF、视频、音频
- Google 文档/表格/幻灯片导出为 PDF 预览；Office 文档调用系统应用
- 右侧"预览 / 详情"面板，含 MD5、文件 ID、位置

### 传输
- 下载：Range 续传、并发可调、进度/速度/ETA、取消与重试、MD5/尺寸校验
- 上传：8 MiB 分块 resumable session、同名自动消歧
- 文件夹下载保持目录结构；传输坞可折叠、可拖高

### 登录
- Google OAuth（loopback + PKCE），刷新令牌存系统凭据库
- 构建可内置 OAuth 客户端，实现一键登录（`pnpm bundle:with-client`）
- 也支持粘贴访问令牌（OAuth Playground，1 小时）与自带客户端 JSON

### 其他
- 两套布局（经典/分隔）、3 套配色、浅色/深色/跟随系统、行密度
- 中英双语；macOS Overlay 标题栏；拖放上传；全局快捷键
- 启动自动检查更新，可从 GitHub Releases 下载并重启安装
