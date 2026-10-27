<script setup lang="ts">
// 链接输入栏。获取中变为红色「停止获取」。

import { computed } from 'vue'
import { extractLink } from '@/utils/link'
import {
  MODE_FETCH_LABEL,
  MODE_HINT,
  MODE_INPUT_READONLY,
  MODE_LABEL,
  MODE_SHOW_USERLIST,
} from '@/types/models'
import type { FetchMode } from '@/types/models'

const url = defineModel<string>('url', { default: '' })

const props = defineProps<{
  mode: FetchMode
  fetching: boolean
  /** 任一页面正在下载 → 禁止点击获取 */
  locked: boolean
}>()

const emit = defineEmits<{
  fetch: []
  stop: []
  openUsers: []
}>()

const hint = computed(() => MODE_HINT[props.mode])

/**
 * 输入或粘贴时立刻抽链接。
 * 用户从抖音复制的通常是「文案 + 短链 + 复制提示」一大段，
 * 直接存整段会让后端解析失败；这里在输入阶段就清理干净。
 */
function onInput(v: string) {
  const link = extractLink(v, props.mode)
  url.value = link || v
}

/** 失焦时再清一次 */
function onBlur() {
  const link = extractLink(url.value, props.mode)
  if (link && link !== url.value) url.value = link
}
const title = computed(() => MODE_LABEL[props.mode])
const readOnly = computed(() => MODE_INPUT_READONLY[props.mode])
const showUserList = computed(() => MODE_SHOW_USERLIST[props.mode])
const fetchLabel = computed(() => MODE_FETCH_LABEL[props.mode])
const inputDisabled = computed(() => props.fetching || props.locked)
</script>

<template>
  <div class="url-bar">
    <span class="mode-tag">
      <el-tag type="primary" effect="plain" size="small">{{ title }}</el-tag>
    </span>

    <el-button
      v-if="showUserList"
      link
      type="primary"
      class="label-btn"
      :disabled="inputDisabled"
      @click="emit('openUsers')"
    >
      用户列表
    </el-button>

    <div v-if="readOnly" class="readonly-tip">
      <el-icon><InfoFilled /></el-icon>
      <span>{{ hint }}</span>
    </div>

    <el-input
      v-else
      v-model="url"
      class="input"
      :placeholder="hint"
      clearable
      :disabled="inputDisabled"
      @update:model-value="onInput"
      @blur="onBlur"
      @keyup.enter="!inputDisabled && emit('fetch')"
    />

    <el-button v-if="!fetching" type="primary" :disabled="locked" @click="emit('fetch')">
      {{ fetchLabel }}
    </el-button>
    <el-button v-else type="danger" class="stop-btn" @click="emit('stop')">
      <el-icon><VideoPause /></el-icon>
      停止获取
    </el-button>
  </div>
</template>

<style scoped>
.url-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}

.mode-tag,
.label-btn {
  flex: 0 0 auto;
}

.input {
  flex: 1;
  min-width: 0;
}

.readonly-tip {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 11px;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  background: #f5f7fa;
  color: #909399;
  font-size: 13px;
}

.stop-btn {
  flex: 0 0 auto;
}
</style>
