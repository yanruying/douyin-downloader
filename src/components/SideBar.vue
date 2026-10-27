<script setup lang="ts">
// 侧栏：登录入口（未登录=按钮，已登录=头像）+ 六个导航项。
// 下载期间禁止切换页面。

import { computed, type Component } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  anyDownloading,
  anyFetching,
  cookieStatus,
  displayName,
  isLoggedIn,
  profile,
} from '@/stores/app'
import type { FetchMode } from '@/types/models'
import { MODE_LABEL } from '@/types/models'
import HeartIcon from '@/components/icons/HeartIcon.vue'

const route = useRoute()
const router = useRouter()

const emit = defineEmits<{
  login: []
  logout: []
}>()

const activePath = computed(() => route.path)
const isDownloadPage = computed(() => route.path === '/')
const currentMode = computed<FetchMode>(() => {
  const q = route.query.mode as FetchMode | undefined
  return q && MODE_LABEL[q] ? q : 'post'
})

/**
 * icon 为字符串时渲染 Element Plus 图标，为组件时直接渲染。
 * 点赞使用自绘心形（Element Plus 无心形图标），收藏使用内置 Star。
 */
type NavIcon = string | Component
const modeItems: { mode: FetchMode; label: string; icon: NavIcon }[] = [
  { mode: 'single', label: '单作品', icon: 'VideoPlay' },
  { mode: 'post', label: '主页作品', icon: 'User' },
  { mode: 'favorite', label: '点赞作品', icon: HeartIcon },
  { mode: 'collect', label: '收藏作品', icon: 'Star' },
]

const pageItems = [
  { path: '/settings', label: '设置', icon: 'Setting' },
  { path: '/about', label: '关于', icon: 'InfoFilled' },
]

/** 下载中锁住所有导航；抓取中只锁模式切换（避免同时打两个接口） */
const navLocked = computed(() => anyDownloading.value)

function goMode(m: FetchMode) {
  if (navLocked.value) {
    ElMessage.warning('下载进行中，无法切换页面')
    return
  }
  if (anyFetching.value && m !== currentMode.value) {
    ElMessage.warning('正在获取中，请先停止再切换')
    return
  }
  router.push({ path: '/', query: { mode: m } })
}

function goPage(p: string) {
  if (navLocked.value) {
    ElMessage.warning('下载进行中，无法切换页面')
    return
  }
  router.push(p)
}

const avatarFailed = computed(() => !profile.value.avatarUrl)
const avatarBroken = computed(() => false)
</script>

<template>
  <aside class="sidebar">
    <div class="auth">
      <el-tooltip
        v-if="isLoggedIn && !avatarFailed && !avatarBroken"
        :content="`${displayName}｜点击退出登录`"
        placement="right"
      >
        <button class="avatar-btn" @click="emit('logout')">
          <img :src="profile.avatarUrl" alt="" class="avatar" />
          <span class="badge" />
        </button>
      </el-tooltip>

      <el-tooltip
        v-else-if="cookieStatus === 'expired'"
        content="Cookie 已过期，点击重新登录"
        placement="right"
      >
        <button class="login-btn expired" @click="emit('login')">
          <el-icon><WarningFilled /></el-icon>
          <span>已过期</span>
        </button>
      </el-tooltip>

      <el-tooltip v-else content="登录抖音" placement="right">
        <button class="login-btn" @click="emit('login')">
          <el-icon><UserFilled /></el-icon>
          <span>登录</span>
        </button>
      </el-tooltip>
    </div>

    <el-divider class="sep" />

    <nav class="nav">
      <button
        v-for="item in modeItems"
        :key="item.mode"
        class="nav-item"
        :class="{
          active: isDownloadPage && currentMode === item.mode,
          locked: navLocked,
        }"
        @click="goMode(item.mode)"
      >
        <el-icon v-if="typeof item.icon === 'string'" :size="17">
          <component :is="item.icon" />
        </el-icon>
        <HeartIcon v-else :size="17" />
        <span>{{ item.label }}</span>
      </button>
    </nav>

    <el-divider class="sep" />

    <nav class="nav">
      <button
        v-for="item in pageItems"
        :key="item.path"
        class="nav-item"
        :class="{ active: activePath === item.path, locked: navLocked }"
        @click="goPage(item.path)"
      >
        <el-icon v-if="typeof item.icon === 'string'" :size="17">
          <component :is="item.icon" />
        </el-icon>
        <HeartIcon v-else :size="17" />
        <span>{{ item.label }}</span>
      </button>
    </nav>

    <div class="spacer" />
  </aside>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  background: #fff;
  border-right: 1px solid #e4e7ed;
  padding: 10px 0 8px;
  overflow: hidden;
}

.auth {
  display: flex;
  justify-content: center;
  padding: 2px 0 6px;
}

.login-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 3px;
  width: 56px;
  padding: 8px 0;
  border: 1px dashed #c0c4cc;
  border-radius: 8px;
  background: #fafafa;
  color: #606266;
  font-size: 10px;
  cursor: pointer;
  transition: all 0.15s;
}

.login-btn:hover {
  border-color: #409eff;
  color: #409eff;
  background: #ecf5ff;
}

.login-btn.expired {
  border-color: #f56c6c;
  color: #f56c6c;
  background: #fef0f0;
}

.avatar-btn {
  position: relative;
  padding: 0;
  border: none;
  background: none;
  cursor: pointer;
  border-radius: 50%;
  line-height: 0;
}

.avatar {
  width: 46px;
  height: 46px;
  border-radius: 50%;
  object-fit: cover;
  border: 2px solid #409eff;
  background: #f5f7fa;
  transition: transform 0.15s;
}

.avatar-btn:hover .avatar {
  transform: scale(1.06);
}

.badge {
  position: absolute;
  right: 0;
  bottom: 0;
  width: 11px;
  height: 11px;
  border-radius: 50%;
  background: #67c23a;
  border: 2px solid #fff;
}

.sep {
  margin: 4px 10px;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.nav-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 3px;
  padding: 9px 2px;
  color: #606266;
  text-decoration: none;
  font-size: 11px;
  line-height: 1.2;
  border: none;
  border-left: 2px solid transparent;
  background: none;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
}

.nav-item:hover {
  background: #f5f7fa;
  color: #409eff;
}

.nav-item.active {
  color: #409eff;
  border-left-color: #409eff;
  background: #ecf5ff;
  font-weight: 500;
}

/* 下载中：视觉上明确不可点 */
.nav-item.locked {
  opacity: 0.4;
  cursor: not-allowed;
}

.nav-item.locked:hover {
  background: none;
  color: #606266;
}

.nav-item.locked.active:hover {
  color: #409eff;
}

.spacer {
  flex: 1;
}
</style>
