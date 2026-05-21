# 抖音主页作品批量下载工具v3.8 - YanRuYing

> 🪶 一款基于 Python + PyQt6 的抖音主页作品批量下载工具，支持视频、图集、实况图下载及 Excel 导出。  
> 本项目完全开源，仅用于学习与研究，禁止任何商业或违法用途。
> ![赞赏码](https://raw.githubusercontent.com/yanruying/YanRuYing/refs/heads/main/pay.png)
>
> 本项目**a_bogus算法**来源于开源项目 [Douyin_TikTok_Download_API](https://github.com/Evil0ctal/Douyin_TikTok_Download_API)
---

## 🧭 功能特点

| 功能模块          | 说明                                                         |
| ----------------- | ------------------------------------------------------------ |
| 🔗 **主页解析**    | 自动解析用户主页或短链接，提取 `sec_user_id`                 |
| 📦 **批量下载**    | 支持批量下载抖音主页所有作品（视频 / 图片 / 实况图）         |
| 💓 **点赞作品**    | 支持下载抖音用户点赞作品                                     |
| 🧠 **自动分类**    | 自动按合集（mix）和日期分类保存                              |
| 📁 **断点续传**    | 自动跳过已存在的文件，避免重复下载                           |
| 🧾 **Excel 导出**  | 支持导出用户作品数据（类型、发布时间、点赞数、合集等）到 Excel |
| ⚙️ **可视化配置**  | 图形化设置界面，可配置线程数、下载路径、Cookie 等            |
| 🪟 **现代化界面**  | PyQt6 界面风格，布局美观，交互流畅                           |
| 🔐 **Cookie 教程** | 内置 Cookie 获取教程，帮助用户快速配置                       |

---

## 🛠️ 环境要求

- **Python 版本**：≥ 3.8  
- **系统支持**：Windows10 , Windows11
- **下载exe文件无环境要求，开箱即用**
---

## 🚀 快速开始

### 1️⃣ 克隆项目

```bash
git clone https://github.com/YanRuYing/douyin-downloader.git
cd douyin-downloader
```

### 2️⃣ 安装依赖

```bash
pip install requests PyQt6 openpyxl
```

### 3️⃣ 运行程序

```bash
python main.py
```

### 4️⃣ 填写 Cookie

- 点击「设置」→ 「查看教程」 按照说明获取你的 Douyin Cookie  
- 填写后点击保存

### 5️⃣ 输入主页链接并下载

- 输入任意抖音主页链接（支持短链）  
- 点击「获取作品」开始解析  
- 自动列出所有视频/图集，可选择下载类型  
- 支持批量下载与导出 Excel

---

## 📊 Excel 导出说明

程序可将抓取到的作品信息导出为 `.xlsx` 文件，包含字段：

| 字段                              | 说明                       |
| --------------------------------- | -------------------------- |
| 类型                              | 视频 / 图集                |
| 发布时间                          | 作品发布时间（自动格式化） |
| 文案                              | 视频文案描述               |
| 合集                              | 所属合集（若有）           |
| 点赞数 / 评论数 / 收藏数 / 分享数 | 互动指标                   |
| 推荐次数                          | 抖音推荐次数               |
| 视频时长                          | 自动计算为“X分钟X秒”       |
| 作品链接                          | 直达网页版作品链接         |

导出文件默认保存在：

```
作品数据Excel/用户名.xlsx
```

---

## ⚙️ 配置文件说明

程序会自动生成 `config.ini`，示例：

```ini
[main]
path = D:\Downloads\Douyin
use_mix_folder = True
include_date_in_filename = True
auto_select_after_fetch = False
threads = 8
cookie = your_cookie_here

[users]
user1 = 张三,https://www.douyin.com/user/MS4wLjABAAAAxxxx
```

---

## 🧩 项目结构

```
.
├─ 主程序.py        # 主程序
├─ config.ini      # 配置文件（自动生成）
├─ 作品数据Excel/   # 导出的 Excel 数据
└─ 用户昵称/      # 下载保存目录
```

---

## 💡 常见问题（FAQ）

**Q1：程序打不开或界面闪退？**  
A：请确认已安装 PyQt6，命令：`pip install PyQt6`

**Q2：提示 Cookie 错误？**  
A：Cookie 具有时效性，请重新获取并更新。

**Q3：如何加快下载速度？**  
A：在设置中调高线程数（建议 ≤8），过高可能被风控。

**Q4：导出 Excel 报错？**  
A：请安装依赖：`pip install openpyxl`

---

## 🖼️ 运行截图

> 以下为实际运行界面示意：

![主界面截图](https://raw.githubusercontent.com/yanruying/douyin-downloader/refs/heads/main/251023013355554.png)
![设置界面](https://raw.githubusercontent.com/yanruying/douyin-downloader/refs/heads/main/251023013445461.png)
![Excel导出](https://raw.githubusercontent.com/yanruying/douyin-downloader/refs/heads/main/251023013738453.png)

---

## 🧑‍💻 作者信息

**作者：** 颜如嘤 (YanRuYing)  
**项目更新页：** [52pojie.cn](https://www.52pojie.cn/thread-2064455-1-1.html)

---

## 📜 LICENSE

```
MIT License

Copyright (c) 2025 颜如嘤

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the “Software”), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, subject to the following conditions:

⚠️ 禁止将本程序用于任何商业、违法、或侵犯隐私的用途
本程序仅供学习与研究，请在合法范围内使用。
```

---

## ⭐ Star 支持

如果你觉得这个项目对你有帮助，请点一个 ⭐ Star 支持作者！  
