<script setup lang="ts">
// 自绘右键菜单。
//
// 浏览器/WebView 的默认右键菜单（后退、刷新、另存为…）在桌面应用里毫无意义，
// 还会挡住操作，所以全局禁用，只在作品列表页换成这三个动作。
//
// 实现要点：
// - 挂在 window 的 contextmenu 上，用 closest 判断是否在表格区域内
// - 菜单位置贴边时自动翻转，避免超出视口
// - 点击其它地方 / Esc / 滚动都关闭

import { onMounted, onUnmounted, ref } from 'vue'

const props = defineProps<{
  /** 只有为 true 时才替换默认菜单（作品列表页） */
  enabled: boolean
  /** 空列表时禁用动作项 */
  disabled: boolean
}>()

const emit = defineEmits<{
  clear: []
  selectAll: []
  invert: []
}>()

const visible = ref(false)
const x = ref(0)
const y = ref(0)
const menuRef = ref<HTMLElement>()

const MENU_W = 132
const MENU_H = 3 * 34 + 8

function open(clientX: number, clientY: number) {
  // 贴边翻转：菜单不能超出窗口
  const maxX = window.innerWidth - MENU_W - 4
  const maxY = window.innerHeight - MENU_H - 4
  x.value = Math.min(clientX, maxX)
  y.value = Math.min(clientY, maxY)
  visible.value = true
}

function close() {
  visible.value = false
}

function onContextMenu(e: MouseEvent) {
  if (!props.enabled) return // 其它页面：什么都不做（默认菜单已在别处禁用）

  const target = e.target as HTMLElement | null
  // 只在作品表格区域内生效；输入框仍允许系统菜单以便粘贴
  const inTable = !!target?.closest('.table-wrap')
  const inInput = !!target?.closest('input, textarea, [contenteditable="true"]')
  if (!inTable || inInput) {
    close()
    return
  }
  e.preventDefault()
  open(e.clientX, e.clientY)
}

function onGlobalContextMenu(e: MouseEvent) {
  // 全局：永远不让默认菜单出现（输入框除外，保留复制粘贴）
  const target = e.target as HTMLElement | null
  const inInput = !!target?.closest('input, textarea, [contenteditable="true"]')
  if (inInput) return
  e.preventDefault()
}

function onMouseDown(e: MouseEvent) {
  if (!visible.value) return
  if (menuRef.value?.contains(e.target as Node)) return
  close()
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') close()
}

function run(action: 'clear' | 'selectAll' | 'invert') {
  close()
  if (props.disabled) return
  if (action === 'clear') emit('clear')
  else if (action === 'selectAll') emit('selectAll')
  else emit('invert')
}

onMounted(() => {
  window.addEventListener('contextmenu', onContextMenu)
  window.addEventListener('contextmenu', onGlobalContextMenu)
  window.addEventListener('mousedown', onMouseDown, true)
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('scroll', close, true)
})

onUnmounted(() => {
  window.removeEventListener('contextmenu', onContextMenu)
  window.removeEventListener('contextmenu', onGlobalContextMenu)
  window.removeEventListener('mousedown', onMouseDown, true)
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('scroll', close, true)
})
</script>

<template>
  <teleport to="body">
    <div
      v-if="visible"
      ref="menuRef"
      class="ctx-menu"
      :style="{ left: `${x}px`, top: `${y}px` }"
    >
      <button class="ctx-item" :disabled="disabled" @click="run('clear')">
        <el-icon :size="14"><Delete /></el-icon>
        清空列表
      </button>
      <button class="ctx-item" :disabled="disabled" @click="run('selectAll')">
        <el-icon :size="14"><Select /></el-icon>
        全选
      </button>
      <button class="ctx-item" :disabled="disabled" @click="run('invert')">
        <el-icon :size="14"><Switch /></el-icon>
        反选
      </button>
    </div>
  </teleport>
</template>

<style scoped>
.ctx-menu {
  position: fixed;
  z-index: 3000;
  min-width: 132px;
  padding: 4px;
  background: #fff;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.12);
  user-select: none;
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 7px 10px;
  border: none;
  border-radius: 4px;
  background: none;
  color: #303133;
  font-size: 13px;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
}

.ctx-item:hover:not(:disabled) {
  background: #ecf5ff;
  color: #409eff;
}

.ctx-item:disabled {
  color: #c0c4cc;
  cursor: not-allowed;
}
</style>
