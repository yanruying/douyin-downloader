<script setup lang="ts">
// 关于页：居中单栏，信息按「标识 → 链接 → 声明」三段组织。
// 徽章用 shields.io 的动态图片（无需额外依赖）。

import { onMounted, ref } from 'vue'
import { api } from '@/api/ipc'

const version = ref('')

onMounted(async () => {
  version.value = await api.appVersion().catch(() => '4.0.0')
})

const links = [
  { label: '博客', url: 'https://bbs.hookyun.cn', icon: 'Link' },
  { label: 'B 站主页', url: 'https://space.bilibili.com/80435723', icon: 'VideoPlay' },
  { label: '开源地址', url: 'https://github.com/yanruying/douyin-downloader', icon: 'Star' },
  { label: 'QQ 群', url: 'https://qm.qq.com/q/zVnIdRBseA', icon: 'ChatDotRound' },
]

function openLink(url: string) {
  api.openPath(url).catch(() => {})
}
</script>

<template>
  <div class="about">
    <div class="wrap">
      <!-- 标题区 -->
      <header class="hero">
        <h1>抖音作品下载工具<span class="ver">v{{ version }}</span></h1>
        <p class="author">作者：颜如嘤 - YanRuYing</p>

        <!-- Shields.io 徽章 -->
        <div class="badges">
          <img
            alt="version"
            src="https://img.shields.io/badge/version-4.0-409eff?style=flat-square"
          />
          <img
            alt="platform"
            src="https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square"
          />
          <img
            alt="stack"
            src="https://img.shields.io/badge/Tauri%202-Rust-orange?style=flat-square"
          />
          <img
            alt="license"
            src="https://img.shields.io/badge/license-MIT-green?style=flat-square"
          />
          <img
            alt="price"
            src="https://img.shields.io/badge/price-free-success?style=flat-square"
          />
        </div>
      </header>

      <!-- 链接卡片 -->
      <section class="links">
        <button v-for="l in links" :key="l.label" class="link-card" @click="openLink(l.url)">
          <el-icon :size="18"><component :is="l.icon" /></el-icon>
          <div class="link-text">
            <span class="link-label">{{ l.label }}</span>
            <span class="link-url">{{ l.url }}</span>
          </div>
          <el-icon class="arrow" :size="14"><TopRight /></el-icon>
        </button>
      </section>

      <!-- 声明 -->
      <footer class="notice">
        <p class="free">本程序完全免费，仅用于学习与研究</p>
        <p class="warn">禁止将本程序用于任何商业或违法用途</p>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.about {
  height: 100%;
  overflow-y: auto;
  display: flex;
  justify-content: center;
  padding: 24px 16px 32px;
}

.wrap {
  width: 100%;
  max-width: 620px;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}

/* 标题 */
.hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.hero h1 {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
  color: #1f2329;
  letter-spacing: 0.3px;
}

.ver {
  margin-left: 6px;
  font-size: 13px;
  font-weight: 500;
  color: #409eff;
  vertical-align: middle;
}

.author {
  margin: 0;
  font-size: 13px;
  color: #606266;
}

.badges {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 6px;
  margin-top: 8px;
}

.badges img {
  height: 20px;
}

/* 链接卡片 */
.links {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 22px;
}

.link-card {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 11px 14px;
  border: 1px solid #e4e7ed;
  border-radius: 8px;
  background: #fff;
  color: #303133;
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  transition: all 0.16s;
}

.link-card:hover {
  border-color: #409eff;
  background: #ecf5ff;
  transform: translateY(-1px);
  box-shadow: 0 3px 10px rgba(64, 158, 255, 0.12);
}

.link-text {
  display: flex;
  flex-direction: column;
  gap: 1px;
  flex: 1;
  min-width: 0;
}

.link-label {
  font-size: 13px;
  font-weight: 500;
}

.link-url {
  font-size: 11px;
  color: #909399;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.arrow {
  color: #c0c4cc;
  flex: 0 0 auto;
}

.link-card:hover .arrow {
  color: #409eff;
}

/* 技术信息 */





/* 声明 */
.notice {
  margin-top: 24px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.notice p {
  margin: 0;
  font-size: 12px;
}

.free {
  color: #67c23a;
  font-weight: 500;
}

.warn {
  color: #f56c6c;
}
</style>
