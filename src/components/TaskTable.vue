<script setup lang="ts">
// 作品表格 + 类型过滤 + 全选/反选。
//
// 交互要求：
//   - 复选框加大（默认 14px 太小，易点错）
//   - Ctrl+A 全选（表格未聚焦时也生效）
//   - 鼠标框选：按住左键拖拽划过行即选中
//
// 框选实现说明：el-table 没有内置框选，这里在滚动容器上监听
// mousedown/mousemove，用行元素自带的 row-key 计算命中集合。
// 拖拽期间禁用文本选择，避免选出一片蓝色文字。

import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import type { TaskRow, TaskKind } from '@/types/models'
import { KIND_LABEL } from '@/types/models'

const props = defineProps<{
  rows: TaskRow[]
  disabled: boolean
}>()

const emit = defineEmits<{
  'update:selected': [ids: string[]]
}>()

const tableRef = ref()
const wrapRef = ref<HTMLElement>()
const typeFilter = ref<TaskKind[]>(['video', 'image', 'live'])
const selected = ref<TaskRow[]>([])

const visibleRows = computed(() => props.rows.filter((r) => typeFilter.value.includes(r.kind)))
const visibleIds = computed(() => new Set(visibleRows.value.map((r) => r.id)))

const kindTag = (k: TaskKind) =>
  ({ video: 'primary', image: 'success', live: 'warning' })[k] as
    | 'primary'
    | 'success'
    | 'warning'

function emitSelected() {
  emit('update:selected', selected.value.map((r) => r.id))
}

/** 以表格自身 selection 事件为唯一真相 */
function onSelectionChange(rows: TaskRow[]) {
  // 过滤后不可见的行不应留在选择里，否则下载会带上用户看不到的作品
  selected.value = rows.filter((r) => visibleIds.value.has(r.id))
  emitSelected()
}

function selectAll() {
  tableRef.value?.clearSelection()
  const all = visibleRows.value
  if (!all.length) return
  all.forEach((row) => tableRef.value?.toggleRowSelection(row, true))
}

function invert() {
  const cur = new Set(selected.value.map((r) => r.id))
  tableRef.value?.clearSelection()
  visibleRows.value
    .filter((r) => !cur.has(r.id))
    .forEach((row) => tableRef.value?.toggleRowSelection(row, true))
}

function clearSelection() {
  tableRef.value?.clearSelection()
  selected.value = []
  emitSelected()
}

// ---------------- Ctrl+A 全选 ----------------
function onKeydown(e: KeyboardEvent) {
  if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== 'a') return
  // 输入框里按 Ctrl+A 应该是选中文本，不是全选表格
  const el = e.target as HTMLElement | null
  const tag = el?.tagName?.toLowerCase()
  if (tag === 'input' || tag === 'textarea' || el?.isContentEditable) return
  if (!visibleRows.value.length) return
  e.preventDefault()
  selectAll()
}

// ---------------- 鼠标框选 ----------------
const dragging = ref(false)
const dragStartY = ref(0)
let dragBase = new Set<string>()

/**
 * 由屏幕坐标定位行号。
 * el-table 渲染的 tbody tr 不带 data-row-key 属性，
 * 因此按 DOM 顺序索引再映射到 visibleRows。
 */
function rowIndexAt(clientY: number): number {
  const rows = wrapRef.value?.querySelectorAll<HTMLElement>('tbody tr')
  if (!rows?.length) return -1
  for (let i = 0; i < rows.length; i++) {
    const r = rows[i].getBoundingClientRect()
    if (clientY >= r.top && clientY <= r.bottom) return i
  }
  return -1
}

/** 拖拽区间内的行索引范围（含端点，自动处理上下方向） */
function rowRange(y1: number, y2: number): [number, number] {
  const lo = Math.min(y1, y2)
  const hi = Math.max(y1, y2)
  const rows = wrapRef.value?.querySelectorAll<HTMLElement>('tbody tr')
  if (!rows?.length) return [-1, -1]
  let first = -1
  let last = -1
  for (let i = 0; i < rows.length; i++) {
    const r = rows[i].getBoundingClientRect()
    const mid = r.top + r.height / 2
    if (mid >= lo && mid <= hi) {
      if (first < 0) first = i
      last = i
    }
  }
  return [first, last]
}

function onMouseDown(e: MouseEvent) {
  // 只处理表格主体内的左键拖拽；点到复选框/表头/横向滚动条就交给原生行为
  if (e.button !== 0 || props.disabled) return
  const target = e.target as HTMLElement
  if (target.closest('.el-checkbox') || target.closest('th') || target.closest('.el-table__header')) {
    return
  }
  const idx = rowIndexAt(e.clientY)
  if (idx < 0) return

  dragging.value = true
  dragStartY.value = e.clientY
  // Ctrl 拖拽 = 在原选择上追加；否则从这次拖拽重新开始
  dragBase = e.ctrlKey || e.metaKey ? new Set(selected.value.map((r) => r.id)) : new Set()
  applyDrag(e.clientY)
  document.body.style.userSelect = 'none'
}

function applyDrag(clientY: number) {
  const [first, last] = rowRange(dragStartY.value, clientY)
  if (first < 0) return

  // DOM 行序与 visibleRows 一一对应（el-table 按 :data 顺序渲染）
  const next = new Set(dragBase)
  for (let i = first; i <= last; i++) {
    const row = visibleRows.value[i]
    if (row) next.add(row.id)
  }

  tableRef.value?.clearSelection()
  visibleRows.value
    .filter((row) => next.has(row.id))
    .forEach((row) => tableRef.value?.toggleRowSelection(row, true))
}

function onMouseMove(e: MouseEvent) {
  if (!dragging.value) return
  e.preventDefault()
  applyDrag(e.clientY)
}

function onMouseUp() {
  if (!dragging.value) return
  dragging.value = false
  document.body.style.userSelect = ''
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup', onMouseUp)
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('mouseup', onMouseUp)
  document.body.style.userSelect = ''
})

// 过滤变化：把不可见行移出选择
watch(typeFilter, () => {
  const kept = selected.value.filter((r) => visibleIds.value.has(r.id))
  if (kept.length === selected.value.length) return
  tableRef.value?.clearSelection()
  kept.forEach((r) => tableRef.value?.toggleRowSelection(r, true))
  selected.value = kept
  emitSelected()
})

// 数据被整体替换（重新抓取/清空）时复位选择
watch(
  () => props.rows,
  () => {
    selected.value = []
    emitSelected()
  },
)

defineExpose({ selectAll, invert, clearSelection })
</script>

<template>
  <div ref="wrapRef" class="table-wrap" @mousedown="onMouseDown">
    <el-table
      ref="tableRef"
      :data="visibleRows"
      height="100%"
      size="small"
      border
      stripe
      row-key="id"
      :empty-text="rows.length ? '当前类型过滤下没有作品' : '还没有作品，先获取一次'"
      @selection-change="onSelectionChange"
    >
      <el-table-column type="selection" width="52" :selectable="() => !disabled" />
      <el-table-column label="#" type="index" width="52" align="center" />
      <el-table-column prop="date" label="发布日期" width="100" align="center" />
      <el-table-column prop="desc" label="描述" min-width="280">
        <template #default="{ row }">
          <div class="cell-wrap">{{ row.desc }}</div>
        </template>
      </el-table-column>
      <el-table-column prop="mix" label="合集" width="110" show-overflow-tooltip />
      <el-table-column label="类型" width="88" align="center" header-align="center">
        <template #header>
          <!-- 用 el-popover + checkbox-group 而不是 el-dropdown:
               dropdown-item 会拦截内部复选框的点击事件，且点击后默认收起面板，
               导致「点了没反应」。popover 不抢事件，适合多选过滤。 -->
          <el-popover
            placement="bottom"
            :width="140"
            trigger="click"
            :disabled="disabled"
          >
            <template #reference>
              <span class="type-header">
                类型
                <el-icon :size="12"><ArrowDown /></el-icon>
              </span>
            </template>
            <el-checkbox-group v-model="typeFilter" class="filter-group">
              <el-checkbox
                v-for="k in (['video', 'image', 'live'] as TaskKind[])"
                :key="k"
                :value="k"
                :label="KIND_LABEL[k]"
              />
            </el-checkbox-group>
          </el-popover>
        </template>
        <template #default="{ row }">
          <el-tag :type="kindTag(row.kind as TaskKind)" size="small" effect="light">
            {{ KIND_LABEL[row.kind as TaskKind] }}
          </el-tag>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<style scoped>
.table-wrap {
  flex: 1;
  min-height: 0;
  /* 拖拽框选时不要选中文本 */
  user-select: none;
}

.type-header {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 2px;
  cursor: pointer;
  color: #606266;
  width: 100%;
  /* 用 line-height 与 th 的高度对齐，而不是 height:100%：
     el-table 的 th 里还套了一层 div.cell，高度撑不满会导致内容贴顶 */
  line-height: 23px;
  vertical-align: middle;
}

.type-header:hover {
  color: #409eff;
}
</style>

<style>
/* 复选框尺寸 20px。el-table 的 size="small" 会附加 .el-checkbox--small，
   其 .el-checkbox__inner 固定为 12px 且权重高于外层容器规则，
   因此用 .el-table__cell 前缀提高权重并显式覆盖 small 变体。 */
.table-wrap .el-table .el-table__cell .el-checkbox__inner,
.table-wrap .el-table .el-checkbox--small .el-checkbox__inner {
  width: 20px !important;
  height: 20px !important;
  border-radius: 3px;
}

.table-wrap .el-table .el-table__cell .el-checkbox__inner::after {
  width: 5px !important;
  height: 10px !important;
  border-width: 2px !important;
}

/* 勾号视觉居中：EP 用 translate(-45%,-60%)，放大后需按比例微调 */
.table-wrap .el-table .el-checkbox__input.is-checked .el-checkbox__inner::after {
  transform: translate(-45%, -60%) rotate(45deg) scaleY(1) translateY(-1px);
}

/* 半选横线按新尺寸居中 */
.table-wrap .el-table .el-checkbox__input.is-indeterminate .el-checkbox__inner::before {
  height: 3px;
  top: 7px;
}

.table-wrap .el-table .el-checkbox {
  height: 26px;
  display: inline-flex;
  align-items: center;
}

.table-wrap .el-table .el-table__cell {
  padding-top: 4px;
  padding-bottom: 4px;
}

.filter-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 2px;
}

.filter-group .el-checkbox {
  margin-right: 0;
  height: 26px;
}
</style>
