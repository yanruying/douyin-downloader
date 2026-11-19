// 全局状态。
//
// 两条铁律：
//   1. 页面切换不能丢任务 —— 列表/进度以**后端**为准，前端只是投影。
//   2. 每个取数模式各有一份独立状态 —— 主页/点赞/收藏/单作品互不影响。

import { computed, ref } from 'vue'
import type {
  CookieStatus,
  DownloadSummary,
  FetchMode,
  ModeState,
  ProfileInfo,
  Settings,
  TaskRow,
} from '@/types/models'
import { emptyModeState } from '@/types/models'

// ---------- 每个模式的状态 ----------
export const modes = ref<Record<FetchMode, ModeState>>({
  single: emptyModeState('single'),
  post: emptyModeState('post'),
  favorite: emptyModeState('favorite'),
  collect: emptyModeState('collect'),
})

export function modeState(m: FetchMode): ModeState {
  return modes.value[m]
}

/** 把后端快照合并进来（保留前端正在用的字段时不覆盖运行标志以外的输入） */
export function applyRuntime(list: ModeState[]) {
  for (const st of list) {
    const m = st.mode as FetchMode
    if (modes.value[m]) {
      modes.value[m] = { ...modes.value[m], ...st }
      resetIdCache(m)
    }
  }
}

export function patchMode(m: FetchMode, patch: Partial<ModeState>) {
  modes.value[m] = { ...modes.value[m], ...patch }
}

/**
 * 增量追加某模式的作品（按 id 去重，保持到达顺序）。
 *
 * 去重集合按模式缓存，不要每页重建——抓几千条时
 * `new Set(tasks.map(...))` 是 O(n²)，会让界面在最后几页直接卡住。
 */
const idCache: Partial<Record<FetchMode, Set<string>>> = {}

function idsOf(m: FetchMode): Set<string> {
  let s = idCache[m]
  if (!s) {
    s = new Set(modes.value[m].tasks.map((t) => t.id))
    idCache[m] = s
  }
  return s
}

/** 列表被整体替换（抓取开始 / 清空 / 恢复快照）后必须重置缓存 */
export function resetIdCache(m: FetchMode) {
  idCache[m] = undefined
}

export function appendTasks(m: FetchMode, rows: TaskRow[]) {
  const cur = modes.value[m]
  if (!rows.length) return
  const known = idsOf(m)
  const fresh: TaskRow[] = []
  for (const t of rows) {
    if (known.has(t.id)) continue
    known.add(t.id)
    fresh.push(t)
  }
  if (fresh.length) {
    // 原地 push：Vue 3 的响应式数组能精确追踪追加，
    // 换成 `{...cur, tasks: concat(...)}` 会让表格整表重渲染。
    cur.tasks.push(...fresh)
  }
}

// ---------- 全局（跨页面共享） ----------
export const globalSpeed = ref(0)
export const cookieStatus = ref<CookieStatus>('empty')
export const profile = ref<ProfileInfo>({
  nickname: '',
  uniqueId: '',
  avatarUrl: '',
  secUid: '',
})
export const settings = ref<Settings | null>(null)

/** 任一页面在下载 → 用来禁路由切换与获取按钮 */
export const anyDownloading = computed(() =>
  Object.values(modes.value).some((m) => m.downloading),
)
export const anyFetching = computed(() =>
  Object.values(modes.value).some((m) => m.fetching),
)
export const downloadingMode = computed<FetchMode | null>(() => {
  const found = Object.entries(modes.value).find(([, m]) => m.downloading)
  return found ? (found[0] as FetchMode) : null
})

export const speedText = computed(() => {
  const v = globalSpeed.value
  if (v < 1024) return `${v} B/s`
  if (v < 1024 * 1024) return `${(v / 1024).toFixed(1)} KB/s`
  return `${(v / 1024 / 1024).toFixed(2)} MB/s`
})

export const isLoggedIn = computed(
  () => cookieStatus.value === 'ok' && !!profile.value.avatarUrl,
)

export const displayName = computed(() => profile.value.nickname)

export function resetMode(m: FetchMode) {
  const url = modes.value[m].url
  modes.value[m] = { ...emptyModeState(m), url }
  resetIdCache(m)
}

export function progressPercent(st: ModeState): number {
  const { completed, total } = st.progress
  if (total <= 0) return 0
  return Math.min(100, Math.max(0, Math.round((completed / total) * 100)))
}

export function progressLabel(st: ModeState): string {
  const { completed, total } = st.progress
  const d: DownloadSummary | null = st.lastDownload
  if (st.downloading) {
    return `${completed} / ${total} (${progressPercent(st)}%)`
  }
  if (!d) return `${completed} / ${total}`
  const parts = [`成功 ${d.ok}`]
  if (d.skipped) parts.push(`跳过 ${d.skipped}`)
  if (d.failed) parts.push(`失败 ${d.failed}`)
  return (d.stopped ? '已停止 · ' : '已完成 · ') + parts.join(' · ')
}

/** 该页面是否应显示底部进度条：只有它自己在下载/刚下完时 */
export function showProgress(st: ModeState): boolean {
  return st.downloading || (st.progress.total > 0 && !!st.lastDownload)
}
