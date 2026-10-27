<script setup lang="ts">
// 应用外壳：侧栏 + 内容 + 关闭流程。
//
// 窗口关闭流程：
//   Rust 拦截 closeRequested 后 emit `window:close-request`，由此处决策：
//   有任务在跑则先确认，确认后停止任务再退出；无任务则询问退出或最小化到托盘。
//   已启用 closeToTray 时跳过询问，直接隐藏。

import { onMounted, onUnmounted, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { api } from '@/api/ipc'
import {
  onCloseRequest,
  onCookieStatus,
  onDownloadDone,
  onFetchPage,
  onFetchDone,
  onLog,
  onProfile,
  onProgress,
  onSpeed,
} from '@/api/events'
import {
  anyDownloading,
  appendTasks,
  applyRuntime,
  cookieStatus,
  globalSpeed,
  modes,
  patchMode,
  profile,
  settings,
} from '@/stores/app'
import type { CloseRequest, CookieStatus, DownloadSummary, FetchMode, LogEvent, ProfileInfo } from '@/types/models'
import SideBar from '@/components/SideBar.vue'
import CookieDialog from '@/components/CookieDialog.vue'
import CloseDialog from '@/components/CloseDialog.vue'

const loginVisible = ref(false)
const unlisteners: Array<() => void> = []
/** 正在退出：忽略后续关闭事件 */
const quitting = ref(false)

async function boot() {
  const init = await api.initApp().catch((e) => {
    ElMessage.error(`初始化失败：${(e as Error).message}`)
    return null
  })
  if (!init) return
  settings.value = init.settings
  cookieStatus.value = init.cookieStatus
  profile.value = init.profile
  applyRuntime(init.modes)
  if (init.legacyImported) ElMessage.success('已从旧版配置导入')
  if (init.cookieStatus === 'expired') ElMessage.warning('Cookie 已过期，请重新登录')
}

onMounted(async () => {
  await boot()

  unlisteners.push(await onSpeed((s) => (globalSpeed.value = s.bytesPerSec)))
  unlisteners.push(await onCookieStatus((s: CookieStatus) => (cookieStatus.value = s)))
  unlisteners.push(await onProfile((p: ProfileInfo) => (profile.value = p)))

  // 这些事件全局都要更新 store，页面组件只负责展示自己那一份
  unlisteners.push(await onFetchPage((p) => appendTasks(p.mode as FetchMode, p.tasks)))
  unlisteners.push(
    await onFetchDone((r) => {
      const m = (r.mode ?? 'post') as FetchMode
      if (modes.value[m]) patchMode(m, { fetching: false })
    }),
  )
  unlisteners.push(
    await onProgress((p) => {
      const m = p.mode as FetchMode
      if (modes.value[m]) patchMode(m, { progress: p })
    }),
  )
  unlisteners.push(
    await onDownloadDone((d: DownloadSummary) => {
      const m = (d.mode || 'post') as FetchMode
      if (modes.value[m]) patchMode(m, { downloading: false, lastDownload: d })
    }),
  )
  unlisteners.push(await onLog((e: LogEvent) => void e))

  unlisteners.push(await onCloseRequest(handleCloseRequest))

  // 定时复查 Cookie：过期时头像自动变回登录按钮
  const timer = window.setInterval(async () => {
    if (!settings.value?.cookie) return
    const s = await api.checkCookie().catch(() => null)
    if (s) {
      cookieStatus.value = s.status
      profile.value = s.profile
    }
  }, 10 * 60 * 1000)
  onUnmounted(() => window.clearInterval(timer))
})

onUnmounted(() => unlisteners.forEach((fn) => fn()))

const closeVisible = ref(false)
const closeBusy = ref(false)
const closeDownloading = ref(false)

async function handleCloseRequest(req: CloseRequest) {
  if (quitting.value || closeVisible.value) return

  // 隐藏窗口不终止任务：后端继续下载，从托盘恢复后可见
  if (settings.value?.closeToTray) {
    await hideToTray()
    ElMessage.success('已最小化到任务栏，下载继续，点击托盘图标可恢复')
    return
  }

  closeBusy.value = req.busy
  closeDownloading.value = req.downloading
  closeVisible.value = true
}

/** 对话框确认：busy 时先结束任务；非 busy 时按勾选决定隐藏还是退出 */
async function onCloseConfirm(minimizeToTray: boolean) {
  if (closeBusy.value) {
    await doQuit()
    return
  }
  if (minimizeToTray) {
    await persistCloseToTray(true)
    await hideToTray()
    ElMessage.success('已最小化到任务栏，点击托盘图标可恢复')
    return
  }
  await persistCloseToTray(false)
  await doQuit()
}

async function persistCloseToTray(v: boolean) {
  if (!settings.value) return
  settings.value.closeToTray = v
  await api.saveSettings(settings.value).catch(() => {})
}

/** 隐藏窗口到托盘（通过后端命令，避免前端直接操作窗口 API） */
async function hideToTray() {
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('hide_to_tray').catch(() => {})
}

async function doQuit() {
  quitting.value = true
  await api.stopAll().catch(() => {})
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke('quit_app').catch(() => {})
}

function onLoginSaved() {
  api.getSettings().then((v) => (settings.value = v))
  api.checkCookie().then((s) => {
    cookieStatus.value = s.status
    profile.value = s.profile
  })
}

async function doLogout() {
  if (anyDownloading.value) {
    ElMessage.warning('下载进行中，无法退出登录')
    return
  }
  try {
    await ElMessageBox.confirm('退出后需要重新扫码登录，已下载的文件不受影响。', '退出登录', {
      type: 'warning',
      confirmButtonText: '退出',
      cancelButtonText: '取消',
    })
  } catch {
    return
  }
  await api.logout().catch(() => {})
  cookieStatus.value = 'empty'
  profile.value = { nickname: '', uniqueId: '', avatarUrl: '', secUid: '' }
  if (settings.value) settings.value.cookie = ''
  ElMessage.success('已退出登录')
}
</script>

<template>
  <div class="app-shell">
    <SideBar @login="loginVisible = true" @logout="doLogout" />

    <div class="main-column">
      <main class="content">
        <router-view />
      </main>
    </div>

    <CookieDialog v-model:visible="loginVisible" @saved="onLoginSaved" />

    <CloseDialog
      v-model="closeVisible"
      :busy="closeBusy"
      :downloading="closeDownloading"
      @confirm="onCloseConfirm"
    />
  </div>
</template>

<style scoped>
.app-shell {
  display: grid;
  grid-template-columns: 84px minmax(0, 1fr);
  height: 100vh;
  overflow: hidden;
  background: #f5f7fa;
}

.main-column {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.content {
  flex: 1;
  min-height: 0;
  padding: 12px;
  overflow: hidden;
}
</style>
