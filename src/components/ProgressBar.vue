<script setup lang="ts">
// 底部进度条：仅显示当前页面的下载进度，下载结束后隐藏网速，空闲时整条收起。

import { computed } from 'vue'
import type { ModeState } from '@/types/models'
import { progressLabel, progressPercent, speedText, showProgress } from '@/stores/app'

const props = defineProps<{
  state: ModeState
}>()

const visible = computed(() => showProgress(props.state))
const percent = computed(() => progressPercent(props.state))
const label = computed(() => progressLabel(props.state))

const status = computed(() => {
  if (props.state.downloading) return undefined
  const d = props.state.lastDownload
  if (d && d.failed > 0) return 'exception' as const
  return 'success' as const
})

/** 网速只在下载进行中显示 */
const showSpeed = computed(() => props.state.downloading)
</script>

<template>
  <transition name="slide-up">
    <footer v-if="visible" class="footer">
      <el-progress
        class="bar"
        :percentage="percent"
        :status="status"
        :stroke-width="12"
        :show-text="false"
        :indeterminate="state.downloading && state.progress.total === 0"
      />
      <span class="counter mono">{{ label }}</span>
      <template v-if="showSpeed">
        <el-divider direction="vertical" />
        <span class="speed mono">{{ speedText }}</span>
      </template>
    </footer>
  </transition>
</template>

<style scoped>
.footer {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 40px;
  padding: 0 14px;
  border-top: 1px solid #e4e7ed;
  background: #fff;
  flex: 0 0 40px;
}

.bar {
  flex: 1;
  min-width: 120px;
}

.bar :deep(.el-progress-bar) {
  padding-right: 0;
}

.counter {
  font-size: 12px;
  color: #606266;
  white-space: nowrap;
  min-width: 150px;
  text-align: right;
}

.speed {
  font-size: 12px;
  color: #303133;
  white-space: nowrap;
  min-width: 76px;
}

.slide-up-enter-active,
.slide-up-leave-active {
  transition: all 0.2s ease;
}

.slide-up-enter-from,
.slide-up-leave-to {
  transform: translateY(100%);
  opacity: 0;
}
</style>
