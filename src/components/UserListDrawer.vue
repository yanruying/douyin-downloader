<script setup lang="ts">
// 用户列表抽屉。
//
// 条目仅保存 { username, secUid }，界面只展示用户名。
// 批量获取由后端 batch_fetch 命令串行处理。

import { computed, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FetchMode, UserEntry } from '@/types/models'
import { api } from '@/api/ipc'
import { extractLink } from '@/utils/link'

const props = defineProps<{
  users: UserEntry[]
  fetching: boolean
  mode: FetchMode
}>()

const visible = defineModel<boolean>('visible', { default: false })

const emit = defineEmits<{
  'update:users': [users: UserEntry[]]
  fetched: []
  /** 单个用户获取：必须在这里声明，否则 Vue 会把 emit 丢弃（链接也就丢了） */
  'fetch-one': [link: string]
}>()

const list = ref<UserEntry[]>([])
const checked = ref<UserEntry[]>([])
const name = ref('')
const linkInput = ref('')
const batchRunning = ref(false)
const adding = ref(false)
const refreshing = ref(false)

watch(
  () => props.users,
  (v) => {
    list.value = v.map((u) => ({ ...u }))
  },
  { immediate: true, deep: true },
)

const canBatch = computed(
  () => checked.value.length > 0 && !props.fetching && !batchRunning.value,
)

function commit() {
  emit(
    'update:users',
    list.value.map((u) => ({ ...u })),
  )
}

/**
 * 添加用户：抽 sec_uid → 查最新昵称 → 入库。
 *
 * 重复添加同一 sec_uid 时**不拒绝**，而是就地更新昵称/覆盖手填备注。
 * 原实现先做去重判断再查昵称，导致「用户改名后再添加」永远被拦下，
 * 列表里一直留着过期昵称——而昵称正是用来区分用户的唯一信息。
 */
async function addUser() {
  const link = extractLink(linkInput.value, 'post')
  const sec = extractSecUid(link)
  if (!sec) {
    ElMessage.warning('没能从这段内容里识别出主页链接')
    return
  }

  const existingIdx = list.value.findIndex((u) => u.secUid === sec)
  const manualName = name.value.trim()

  // 自动拉最新昵称：用户可能粘贴很久以前的链接，或手写错名字。
  // 查询失败不阻断添加，退回手填名称；手填也没有就沿用列表里的旧值。
  adding.value = true
  let fetched = ''
  let failedMsg = ''
  try {
    const p = await api.lookupProfile(sec)
    fetched = p.nickname || ''
  } catch (e) {
    failedMsg = (e as Error).message
  } finally {
    adding.value = false
  }

  if (existingIdx >= 0) {
    // 已存在：用最新昵称覆盖；拉取失败则保留手填，再退回原值
    const prev = list.value[existingIdx]
    const next = fetched || manualName || prev.username
    list.value[existingIdx] = { username: next || '未命名', secUid: sec }
    if (fetched && fetched !== prev.username) {
      ElMessage.success(`已更新昵称：${prev.username} → ${fetched}`)
    } else {
      ElMessage.info('该用户已在列表中，已刷新信息')
    }
  } else {
    if (failedMsg) {
      ElMessage.warning(`未能获取用户名，先用手填名称：${failedMsg}`)
    }
    list.value.push({
      username: fetched || manualName || '未命名',
      secUid: sec,
    })
  }

  name.value = ''
  linkInput.value = ''
  commit()
}

/// 批量刷新所有已保存用户的昵称。
async function refreshAllNames() {
  if (!list.value.length) {
    ElMessage.info('列表为空')
    return
  }
  refreshing.value = true
  let changed = 0
  let failed = 0
  try {
    for (let i = 0; i < list.value.length; i++) {
      const u = list.value[i]
      try {
        const p = await api.lookupProfile(u.secUid)
        if (p.nickname && p.nickname !== u.username) {
          list.value[i] = { ...u, username: p.nickname }
          changed += 1
        }
      } catch {
        failed += 1
      }
      // 逐个查询之间留间隔，降低触发风控的概率
      await new Promise((r) => setTimeout(r, 300))
    }
    commit()
    if (changed) ElMessage.success(`已更新 ${changed} 个昵称`)
    else ElMessage.info('所有昵称都是最新的')
    if (failed) ElMessage.warning(`${failed} 个用户查询失败（可能触发风控，稍后再试）`)
  } finally {
    refreshing.value = false
  }
}

function extractSecUid(url: string): string {
  const m = url.match(/\/user\/([A-Za-z0-9_\-.]+)/)
  if (m) return m[1]
  const raw = url.match(/(MS4wLjAB[A-Za-z0-9_-]+)/)
  return raw ? raw[1] : ''
}

async function removeUser(idx: number) {
  const u = list.value[idx]
  try {
    await ElMessageBox.confirm(`确定删除「${u.username}」？`, '确认', { type: 'warning' })
    list.value.splice(idx, 1)
    commit()
  } catch {
    /* 取消 */
  }
}

/** 批量获取：交给后端串行抓取，结果合并到当前页面 */
async function batchFetch() {
  if (!canBatch.value) return
  const urls = checked.value.map((u) => `https://www.douyin.com/user/${u.secUid}`)
  batchRunning.value = true
  try {
    await api.batchFetch(urls, props.mode)
    ElMessage.success(`已开始批量获取 ${urls.length} 个用户`)
    emit('fetched')
    visible.value = false
  } catch (e) {
    ElMessage.error((e as Error).message)
  } finally {
    batchRunning.value = false
  }
}

function fetchOne(u: UserEntry) {
  if (!u.secUid) return
  emit('fetch-one', `https://www.douyin.com/user/${u.secUid}`)
}

defineExpose({ fetchOne })
</script>

<template>
  <el-drawer v-model="visible" title="用户列表" size="460px">
    <div class="add-row">
      <el-input v-model="name" placeholder="备注（可选）" class="name-input" />
      <el-input
        v-model="linkInput"
        placeholder="粘贴主页链接或分享文案"
        class="url-input"
        @keyup.enter="addUser"
      />
      <el-button type="primary" :loading="adding" @click="addUser">添加</el-button>
    </div>

    <el-table
      v-if="list.length"
      class="user-table"
      :data="list"
      border
      height="calc(100vh - 250px)"
      @selection-change="(rows: UserEntry[]) => (checked = rows)"
    >
      <el-table-column type="selection" width="46" />
      <el-table-column prop="username" label="用户名" min-width="180" show-overflow-tooltip />
      <el-table-column label="操作" width="112" align="center">
        <template #default="{ row, $index }">
          <el-button link type="primary" :disabled="fetching" @click="fetchOne(row)">
            获取
          </el-button>
          <el-button link type="danger" @click="removeUser($index)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-empty v-else description="还没有保存的用户" :image-size="80" />

    <template #footer>
      <div class="footer">
        <span class="text-muted">共 {{ list.length }} 个</span>
        <div class="spacer" />
        <el-button
          :disabled="!list.length || refreshing"
          :loading="refreshing"
          @click="refreshAllNames"
        >
          刷新昵称
        </el-button>
        <el-button :disabled="!canBatch" type="primary" @click="batchFetch">
          批量获取{{ checked.length ? ` (${checked.length})` : '' }}
        </el-button>
      </div>
    </template>
  </el-drawer>
</template>

<style>
/* 与作品列表一致的复选框尺寸。
   el-table 的 size="small" 会附加 .el-checkbox--small（内部固定 12px），
   故此处不使用 small 尺寸，并以具体选择器覆盖。 */
.user-table .el-table__cell .el-checkbox__inner,
.user-table .el-checkbox--small .el-checkbox__inner {
  width: 20px !important;
  height: 20px !important;
  border-radius: 3px;
}

.user-table .el-table__cell .el-checkbox__inner::after {
  width: 5px !important;
  height: 10px !important;
  border-width: 2px !important;
}

.user-table .el-checkbox__input.is-checked .el-checkbox__inner::after {
  transform: translate(-45%, -60%) rotate(45deg) scaleY(1) translateY(-1px);
}

.user-table .el-checkbox {
  height: 26px;
  display: inline-flex;
  align-items: center;
}

.user-table .el-table__cell {
  padding-top: 4px;
  padding-bottom: 4px;
}
</style>

<style scoped>
.add-row {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.name-input {
  flex: 0 0 110px;
}

.url-input {
  flex: 1;
}

.footer {
  display: flex;
  align-items: center;
}

.spacer {
  flex: 1;
}
</style>
