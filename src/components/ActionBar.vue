<script setup lang="ts">
// 操作按钮行。下载中统一变红「停止下载」；下载期间禁止切换页面与点获取。

const props = defineProps<{
  hasTasks: boolean
  downloading: boolean
  fetching: boolean
  selectedCount: number
  /** 任一页面在下载：锁定所有会干扰的操作 */
  locked: boolean
}>()

const emit = defineEmits<{
  selectAll: []
  invert: []
  clear: []
  exportUrls: []
  exportExcel: []
  download: []
  stop: []
}>()

const busy = () => props.downloading || props.fetching
</script>

<template>
  <div class="action-bar">
    <el-button :disabled="!hasTasks" @click="emit('exportUrls')">导出直链</el-button>
    <el-button :disabled="!hasTasks" @click="emit('exportExcel')">导出 Excel</el-button>

    <div class="spacer" />

    <el-button :disabled="!hasTasks || busy()" @click="emit('clear')">清空列表</el-button>
    <el-button :disabled="!hasTasks || busy()" @click="emit('selectAll')">全选</el-button>
    <el-button :disabled="!hasTasks || busy()" @click="emit('invert')">反选</el-button>

    <el-button
      v-if="!downloading"
      type="primary"
      :disabled="selectedCount === 0 || locked"
      @click="emit('download')"
    >
      开始下载{{ selectedCount ? ` (${selectedCount})` : '' }}
    </el-button>
    <el-button v-else type="danger" @click="emit('stop')">
      <el-icon><VideoPause /></el-icon>
      停止下载
    </el-button>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}

.action-bar :deep(.el-button + .el-button) {
  margin-left: 0;
}

.spacer {
  flex: 1;
}
</style>
