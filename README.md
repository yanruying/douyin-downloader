# 抖音作品下载工具 v4.0

基于 Tauri 2 与 Rust 的抖音作品批量下载桌面应用。支持主页作品、点赞作品、收藏作品与单作品四种抓取模式，内置断点续传、全局网速显示、系统托盘与 Excel 导出。

下载、网速采样、Cookie 管理与抖音接口请求全部在 Rust 进程内完成，不依赖 aria2、ffmpeg 等任何外部程序。

作者：颜如嘤 - YanRuYing

## 功能

| 模块 | 说明 |
| --- | --- |
| 四种抓取模式 | 单作品（视频/图集）、主页作品、点赞作品、收藏作品 |
| 扫码登录 | 内嵌 WebView 打开抖音，自动读取 Cookie，无需手动复制 |
| 批量下载 | 支持多用户批量获取，各用户作品分目录存放 |
| 断点续传 | 中断后保留 `.tmp`，下次从断点继续 |
| 全局网速 | 采样系统网卡接收速率，500ms 刷新 |
| 分类保存 | 按用户、合集、日期自动组织目录结构 |
| 导出 | 视频直链 txt、作品数据 Excel（含点赞/评论/收藏等指标） |
| 用户列表 | 保存常用主页，一键批量获取，自动同步最新昵称 |
| 系统托盘 | 关闭时最小化到托盘，后台继续下载 |
| 类型过滤 | 按视频/图片/实况筛选，支持全选、反选、鼠标框选 |

![主界面截图](https://github.com/yanruying/douyin-downloader/blob/main/1.png)
![设置界面](https://github.com/yanruying/douyin-downloader/blob/main/2.png)
![关于页面截图](https://github.com/yanruying/douyin-downloader/blob/main/3.png)

## 技术栈

- 桌面框架：Tauri 2
- 后端：Rust（reqwest、tokio、sysinfo、rust_xlsxwriter）
- 前端：Vue 3 + TypeScript + Vite
- UI 组件库：Element Plus

## 构建环境要求

- Windows 10 / 11
- Node.js 20 以上
- pnpm 9 以上
- Rust 1.77 以上（含 MSVC 工具链）
- WebView2 运行时（Windows 10/11 通常已预装）

## 开发

```bash
pnpm install
pnpm tauri dev
```

## 打包

```bash
pnpm tauri build
```

产物位置：

```
src-tauri/target/release/douyin-downloader.exe                          免安装版
src-tauri/target/release/bundle/nsis/*_x64-setup.exe                   NSIS 安装包
```

## 测试

```bash
# 后端
cd src-tauri
cargo test

# 前端
pnpm test
```

需要联网与有效 Cookie 的集成测试默认跳过，手动执行：

```bash
cargo test --test douyin_live -- --ignored --nocapture
cargo test --test self_fetch_live -- --ignored --nocapture
```

## 使用方法

1. 点击侧栏的「登录」，扫码完成登录（Cookie 会自动保存）
2. 在侧栏选择抓取模式
3. 粘贴链接或留空获取自己的作品，点击「获取作品」
4. 在列表中勾选需要的作品，点击「开始下载」

### 链接格式

程序会自动从粘贴内容中提取链接，可以直接粘贴抖音的分享文案。

| 模式 | 支持的链接 |
| --- | --- |
| 单作品 | `https://www.douyin.com/video/{id}`、`/note/{id}`、`v.douyin.com` 短链 |
| 主页作品 | `https://www.douyin.com/user/{sec_uid}`、短链 |
| 点赞作品 | 同上 |
| 收藏作品 | 仅当前登录账号，无需填写链接 |

### 关于收藏作品

抖音的收藏接口仅允许查询当前登录账号。传入他人 `sec_uid` 会返回 `status_code=3002279`，与风控无关，冷却后重试结果一致。因此收藏模式固定抓取本人收藏。

## 文件保存结构

根目录默认为程序所在目录，可在设置中修改。程序目录不可写时自动回退到 `文档/DouyinDownloader`。

```
<根目录>/
├─ settings.json                    配置（Cookie、路径、线程数等）
└─ 作品下载/
   └─ <昵称>-<抖音号>[-like|-collect]/
      ├─ .sec_uid_<sec_user_id>     身份标记（空文件，用于识别同一账号）
      ├─ 2025-03-04_作品文案.mp4     视频
      ├─ 合集名称/                   合集分文件夹开启时
      │  └─ 2025-03-04_作品文案.mp4
      └─ images/                    图片与实况图
         ├─ 2025-03-04_作品文案_p1.jpeg
         └─ 2025-03-04_作品文案_live1.mp4
```

其他输出：

```
<根目录>/
├─ 作品数据Excel/<昵称>-<抖音号>.xlsx
└─ 视频直链/<昵称>-<抖音号>.txt
```

后缀说明：`-like` 为点赞作品，`-collect` 为收藏作品，主页与单作品无后缀。

## 设置项

| 设置 | 说明 |
| --- | --- |
| 下载根目录 | 留空则使用程序所在目录 |
| 并发线程数 | 默认 4，上限 8。过高容易触发风控 |
| 合集分文件夹 | 有合集的作品放入「合集名」子目录 |
| 文件名加日期前缀 | 形如 `2025-03-04_文案.mp4` |
| 获取后自动全选 | 抓取完成后自动勾选全部作品 |
| 导出直链带标题 | 开启后每行包含序号与文案 |
| 关闭时最小化到任务栏 | 关闭窗口后隐藏到系统托盘，下载继续 |

### 重复文件处理

下载前会向服务端请求文件大小（HEAD 请求），若目标目录内已存在相同大小的文件，则视为同一作品，仅按当前设置移动或改名，不重新下载。

这一机制的作用是：改动「合集分文件夹」或「文件名加日期」后再次下载，已有文件会被移动到新位置或改名，不会产生重复下载。

需要注意，两个不同的作品若文件大小完全相同，会被判定为同一文件而跳过。抖音视频大小精确到字节相同的概率很低，但纯色图片等场景理论上可能出现。

## 项目结构

```
├─ src/                         前端
│  ├─ api/                      IPC 封装与事件订阅
│  ├─ components/               UI 组件
│  ├─ stores/                   全局状态
│  ├─ views/                    下载、设置、关于
│  ├─ types/                    与 Rust 对应的类型定义
│  └─ utils/                    链接提取等工具
├─ src-tauri/                   后端
│  ├─ src/
│  │  ├─ douyin/                抖音接口层（唯一允许出现接口细节的地方）
│  │  │  ├─ sign/               a_bogus 签名算法
│  │  │  ├─ api.rs              接口调用与分页
│  │  │  ├─ model.rs            响应结构
│  │  │  └─ parser.rs           作品解析
│  │  ├─ download/              下载队列、单文件下载、断点续传
│  │  ├─ auth/                  登录窗口与 Cookie 读取
│  │  ├─ export/                直链与 Excel 导出
│  │  ├─ net/                   网速采样
│  │  ├─ commands.rs            Tauri 命令（薄壳）
│  │  ├─ state.rs               全局状态
│  │  └─ config.rs              配置读写
│  └─ tests/                    集成测试与签名对拍
└─ tests/                       前端测试
```

## 实现说明

### a_bogus 签名

抖音接口需要 `a_bogus` 参数，算法包含 SM3 哈希、自定义 Base64 字母表与 RC4 置换。本项目的实现与参考实现（[f2](https://github.com/Johnserf-Seed/f2)，Apache-2.0）逐字节一致，通过 `tests/golden_abogus.rs` 固定时间戳与随机数进行对拍验证。

算法内部含随机填充与毫秒时间戳，同一参数两次调用结果不同，因此测试只断言结构与长度，不比对完整字符串。

### 前后端通信

命令层只做参数转换，长任务立即返回，进度通过事件推送：

| 事件 | 用途 | 节流 |
| --- | --- | --- |
| `fetch:page` | 每页解析结果，支持边抓边显示 | 每页一次 |
| `progress:update` | 下载进度 | 300ms |
| `speed:update` | 全局网速 | 500ms |
| `download:done` | 下载结果与失败明细 | 结束时一次 |

任务状态保存在 Rust 侧，切换页面不会丢失列表或中断下载。

### 接口注意事项

以下限制来自实际接口行为：

- 收藏接口的 `count` 上限为 20，超出返回空数据
- 收藏接口在「无更多数据」时仍可能返回非零 `status_code`，需先取数据再判断错误码
- 收藏接口的 `cursor` 不前进时必须终止循环，否则会重复抓取同一页
- 风控错误码（如 `3002279`、`2150`）会自行恢复，程序会做一次退避重试并给出可读提示

## 常见问题

**启动后显示空白页或连接被拒绝**

开发模式下程序需要连接 Vite 服务。请使用 `pnpm tauri dev` 启动，或使用 `pnpm tauri build` 后的产物。

**Cookie 多久失效**

抖音 Cookie 有效期不固定，失效后侧栏头像会变回登录按钮，重新扫码即可。

**下载速度慢**

适当提高并发线程数，但建议不超过 8，过高会触发风控。网速显示的是系统全局接收速率，不是本程序单独占用。

**提示请求过于频繁**

抖音风控限流，等待 1~2 分钟再试。期间不要连续点击，否则会延长冷却时间。

## 开源协议

MIT License，详见 [LICENSE](LICENSE)。

a_bogus 签名算法源自 [f2](https://github.com/Johnserf-Seed/f2)（Apache License 2.0）。

## 声明

本程序完全免费，仅用于学习与研究。

禁止将本程序用于任何商业或违法用途。
