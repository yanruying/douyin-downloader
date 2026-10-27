<script setup lang="ts">
// 下载页：四种模式共用，靠路由 query.mode 区分。
// 列表、进度、运行标志都取自该模式独立的那一份 store 状态。

import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { api } from '@/api/ipc'
import {
  onDownloadDone,
  onFetchDone,
  onFetchPage,
  onFetchProfile,
  onLog,
  onProgress,
  onSpeed,
  onWindowFocused,
} from '@/api/events'
import {
  appendTasks,
  anyDownloading,
  globalSpeed,
  modeState,
  modes,
  patchMode,
  resetMode,
  settings,
} from '@/stores/app'
import type { FetchMode, LogEvent, UserEntry } from '@/types/models'
import {
  MODE_ALLOW_SELF,
  MODE_HINT,
  MODE_INPUT_READONLY,
  MODE_LABEL,
  MODE_SHOW_USERLIST,
} from '@/types/models'
import UrlBar from '@/components/UrlBar.vue'
import ActionBar from '@/components/ActionBar.vue'
import TaskTable from '@/components/TaskTable.vue'
import ProgressBar from '@/components/ProgressBar.vue'
import ContextMenu from '@/components/ContextMenu.vue'
import UserListDrawer from '@/components/UserListDrawer.vue'

const route = useRoute()

const selectedIds = ref<string[]>([])
const usersVisible = ref(false)
const logs = ref<{ level: string; text: string; at: string }[]>([])
const logVisible = ref(false)
const tableRef = ref<InstanceType<typeof TaskTable>>()
const unlisteners: Array<() => void> = []

const mode = computed<FetchMode>(() => {
  const q = route.query.mode as FetchMode | undefined
  return q && MODE_LABEL[q] ? q : 'post'
})

const st = computed(() => modeState(mode.value))
const fetching = computed(() => st.value.fetching)
const downloading = computed(() => st.value.downloading)
const tasks = computed(() => st.value.tasks)
const summary = computed(() => st.value.summary)
const hasTasks = computed(() => tasks.value.length > 0)
const showUserList = computed(() => MODE_SHOW_USERLIST[mode.value])
/** 任一页面正在下载时，本页的操作一并锁住 */
const locked = computed(() => anyDownloading.value)

const url = computed({
  get: () => st.value.url,
  set: (v: string) => patchMode(mode.value, { url: v }),
})

// 换模式时重置选择与日志（各页面互不干扰）
watch(
  mode,
  () => {
    selectedIds.value = []
    logs.value = []
  },
  { immediate: true },
)

function pushLog(e: LogEvent) {
  const at = new Date().toLocaleTimeString('zh-CN', { hour12: false })
  logs.value.push({ level: e.level, text: e.text, at })
  if (logs.value.length > 800) logs.value.splice(0, logs.value.length - 800)
}

onMounted(async () => {
  unlisteners.push(
    await onSpeed((s) => {
      globalSpeed.value = s.bytesPerSec
    }),
  )
  unlisteners.push(await onLog(pushLog))

  // 抓取开始就拿到用户名 → 立刻写进该页 summary，状态栏右下角立即显示
  unlisteners.push(
    await onFetchProfile((p) => {
      const m = p.mode
      patchMode(m, {
        summary: {
          nickname: p.nickname,
          uniqueId: p.uniqueId,
          secUid: modes.value[m]?.summary.secUid ?? '',
          count: modes.value[m]?.summary.count ?? 0,
          mode: m,
        },
      })
    }),
  )

  // 每页到达即追加到对应模式
  unlisteners.push(
    await onFetchPage((p) => {
      const m = p.mode as FetchMode
      appendTasks(m, p.tasks)
      if (m === mode.value) {
        pushLog({ level: 'info', text: `第 ${p.page} 页：+${p.added} 个文件` })
      }
    }),
  )

  unlisteners.push(
    await onFetchDone((r) => {
      const m = (r.mode ?? mode.value) as FetchMode
      patchMode(m, { fetching: false })
      const err = (r as { error?: string }).error
      const stopped = (r as { stopped?: boolean }).stopped
      if (m !== mode.value) return
      if (err && !stopped) {
        ElMessage.error(err)
      } else if (typeof r.count === 'number' && !err) {
        ElMessage.success(`${MODE_LABEL[m]}获取完成：${r.count} 个文件`)
        refreshUsers()
        if (settings.value?.autoSelectAfterFetch) {
          setTimeout(() => tableRef.value?.selectAll(), 60)
        }
      }
    }),
  )

  unlisteners.push(
    await onProgress((p) => {
      const m = p.mode as FetchMode
      if (m !== mode.value) return
      patchMode(m, { progress: p })
    }),
  )

  unlisteners.push(
    await onDownloadDone((d) => {
      const m = (d.mode || mode.value) as FetchMode
      patchMode(m, { downloading: false, lastDownload: d })
      if (m !== mode.value) return
      if (d.stopped) {
        ElMessage.info(`已停止：成功 ${d.ok}，跳过 ${d.skipped}，失败 ${d.failed}`)
      } else if (d.failed > 0) {
        ElMessage.warning(`完成：成功 ${d.ok}，跳过 ${d.skipped}，失败 ${d.failed}`)
      } else {
        ElMessage.success(`下载完成：成功 ${d.ok}，跳过 ${d.skipped}`)
      }
      // 失败明细也进日志面板，方便复制排查
      for (const [desc, reason] of d.failures ?? []) {
        pushLog({ level: 'error', text: `失败：${desc} —— ${reason}` })
      }
    }),
  )

  // 恢复后端状态（切页回来 / 热重载）
  const resync = async () => {
    const snap = await api.getRuntime().catch(() => null)
    if (snap) {
      const { applyRuntime } = await import('@/stores/app')
      applyRuntime(snap)
    }
  }
  await resync()

  // 从托盘恢复时再同步一次：隐藏期间 WebView 可能被节流，
  // 事件未必全部到达，恢复后要以后端状态为准。
  unlisteners.push(await onWindowFocused(() => void resync()))
})

onUnmounted(() => {
  unlisteners.forEach((fn) => fn())
})

async function doFetch() {
  if (MODE_INPUT_READONLY[mode.value]) patchMode(mode.value, { url: '' })
  const u = st.value.url.trim()
  if (!u && !MODE_ALLOW_SELF[mode.value]) {
    ElMessage.warning(MODE_HINT[mode.value])
    return
  }
  logs.value = []
  resetMode(mode.value)
  selectedIds.value = []
  try {
    await api.startFetch(u, mode.value)
    patchMode(mode.value, { fetching: true, url: u })
  } catch (e) {
    ElMessage.error((e as Error).message)
  }
}

async function doStopFetch() {
  await api.stopFetch(mode.value).catch(() => {})
  ElMessage.info('已请求停止获取')
}

async function doDownload() {
  try {
    const res = await api.startDownload(mode.value, selectedIds.value)
    if (res.downloading) {
      patchMode(mode.value, {
        downloading: true,
        lastDownload: null,
        progress: {
          mode: mode.value,
          completed: 0,
          total: selectedIds.value.length,
        },
      })
      ElMessage.info('已开始下载，下载期间无法切换页面')
    }
  } catch (e) {
    ElMessage.error((e as Error).message)
  }
}

async function doStopDownload() {
  await api.stopDownload(mode.value).catch(() => {})
  ElMessage.info('已请求停止下载，正在收尾')
}

async function doExportUrls() {
  try {
    const r = await api.exportUrls(mode.value)
    ElMessageBox.alert(r.path, '直链已导出', {
      confirmButtonText: '打开目录',
      callback: () => api.revealPath(r.path).catch(() => {}),
    })
  } catch (e) {
    ElMessage.error((e as Error).message)
  }
}

async function doExportExcel() {
  try {
    const r = await api.exportExcel(mode.value)
    ElMessageBox.alert(r.path, 'Excel 已导出', {
      confirmButtonText: '打开文件',
      callback: () => api.openPath(r.path).catch(() => {}),
    })
  } catch (e) {
    ElMessage.error((e as Error).message)
  }
}

async function doClear() {
  try {
    await ElMessageBox.confirm('确定清空当前列表？', '确认', { type: 'warning' })
    await api.clearTasks(mode.value)
    resetMode(mode.value)
    selectedIds.value = []
  } catch {
    /* 取消 */
  }
}

/** 用户列表增删后立刻持久化，并同步到全局设置 */
async function onUsersChanged(list: UserEntry[]) {
  if (!settings.value) return
  settings.value = { ...settings.value, users: list }
  await api.saveSettings(settings.value).catch((e) =>
    ElMessage.error(`保存用户列表失败：${(e as Error).message}`),
  )
}

/**
 * 后端抓取成功后会把它自己记进用户列表并落盘，
 * 这里在抓取结束时重新拉一次设置，让抽屉立刻看到新用户。
 */
/**
 * 批量获取启动后同步前端状态：
 * 清空列表并置 fetching=true，使「停止获取」按钮可用。
 */
async function onBatchStarted() {
  resetMode(mode.value)
  selectedIds.value = []
  logs.value = []
  patchMode(mode.value, { fetching: true })
  refreshUsers()
}

/** 从用户列表里单个获取：把链接填进输入框再走正常流程 */
async function onFetchOneUser(link: string) {
  url.value = link
  await doFetch()
  usersVisible.value = false
}

async function refreshUsers() {
  const s = await api.getSettings().catch(() => null)
  if (s) settings.value = s
}

/** 打开用户列表抽屉。单作品模式没有用户列表，提示而不是静默无反应。 */
function openUsers() {
  if (!showUserList.value) {
    ElMessage.info('单作品模式不需要用户列表，请粘贴作品链接')
    return
  }
  usersVisible.value = true
}

const lastLog = computed(() => logs.value[logs.value.length - 1])

/**
 * 右下角显示的用户名。
 * 抓取一开始后端就把昵称推过来了，所以这里在 fetching 期间也有值；
 * 若还没拿到（极短的请求），退回显示上次的结果昵称。
 */
const userLabel = computed(() => summary.value.nickname || '')
const countLabel = computed(() =>
  summary.value.count ? `共 ${summary.value.count} 个作品` : '',
)
</script>

<template>
  <div class="download-view">
    <UrlBar
      v-model:url="url"
      :mode="mode"
      :fetching="fetching"
      :locked="locked && !downloading"
      @fetch="doFetch"
      @stop="doStopFetch"
      @open-users="openUsers"
    />

    <ActionBar
      :has-tasks="hasTasks"
      :downloading="downloading"
      :fetching="fetching"
      :selected-count="selectedIds.length"
      :locked="locked && !downloading"
      @select-all="tableRef?.selectAll()"
      @invert="tableRef?.invert()"
      @clear="doClear"
      @export-urls="doExportUrls"
      @export-excel="doExportExcel"
      @download="doDownload"
      @stop="doStopDownload"
    />

    <TaskTable
      ref="tableRef"
      :rows="tasks"
      :disabled="downloading"
      @update:selected="(ids) => (selectedIds = ids)"
    />

    <!-- 状态行：抓取中显示进度，空闲时显示最后一条日志 -->
    <div class="status-strip" @click="logVisible = true">
      <template v-if="fetching">
        <el-icon class="is-loading"><Loading /></el-icon>
        <span>正在获取{{ MODE_LABEL[mode] }}… 已获取 {{ tasks.length }} 个</span>
      </template>
      <template v-else>
        <el-icon><Document /></el-icon>
        <span class="log-text">{{ lastLog ? lastLog.text : '日志' }}</span>
      </template>
      <div class="spacer" />
      <!-- 右下角：当前用户名 + 数量。抓取中也显示（昵称在分页前就到了） -->
      <span v-if="userLabel" class="user-label">
        <el-icon :size="13"><User /></el-icon>
        {{ userLabel }}
      </span>
      <span v-if="countLabel" class="text-muted">{{ countLabel }}</span>
      <span v-else-if="fetching && tasks.length" class="text-muted">
        已取 {{ tasks.length }} 个
      </span>
    </div>

    <ProgressBar :state="st" />

    <!-- 作品列表页专属右键菜单；其它页面由组件内部判断后不替换默认行为 -->
    <ContextMenu
      :enabled="true"
      :disabled="!hasTasks || downloading"
      @clear="doClear"
      @select-all="tableRef?.selectAll()"
      @invert="tableRef?.invert()"
    />

    <UserListDrawer
      ref="userDrawerRef"
      v-model:visible="usersVisible"
      :users="settings?.users ?? []"
      :fetching="fetching"
      :mode="mode"
      @update:users="onUsersChanged"
      @fetched="onBatchStarted"
      @fetch-one="onFetchOneUser"
    />

    <el-drawer v-model="logVisible" title="运行日志" size="460px">
      <div class="log-list">
        <div v-for="(l, i) in logs" :key="i" class="log-item" :class="l.level">
          <span class="mono log-time">{{ l.at }}</span>
          <span class="selectable">{{ l.text }}</span>
        </div>
        <el-empty v-if="!logs.length" description="暂无日志" :image-size="60" />
      </div>
    </el-drawer>
  </div>
</template>

<style scoped>
.download-view {
  display: flex;
  flex-direction: column;
  gap: 10px;
  height: 100%;
  min-height: 0;
}

.status-strip {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #606266;
  cursor: pointer;
  flex: 0 0 auto;
  padding: 0 2px;
  min-height: 18px;
}

.status-strip:hover {
  color: #409eff;
}

.log-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 42%;
}

.spacer {
  flex: 1;
}

.user-label {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  color: #303133;
  font-weight: 500;
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-list {
  font-size: 12px;
}

.log-item {
  display: flex;
  gap: 8px;
  padding: 3px 0;
  border-bottom: 1px solid #f2f6fc;
  line-height: 1.5;
}

.log-item.error {
  color: #f56c6c;
}

.log-item.warn {
  color: #e6a23c;
}

.log-time {
  color: #c0c4cc;
  flex: 0 0 auto;
}
</style>
