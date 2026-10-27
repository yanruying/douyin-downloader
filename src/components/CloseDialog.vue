<script setup lang="ts">
// 退出确认对话框。
//
// 不用 ElMessageBox + h() 拼 VNode：那种写法里 ElCheckbox 的 v-model
// 拿不到 Vue 的响应式上下文，勾选后状态不会回流，表现为「点不动」。
// 这里用真实的模板，状态是组件内的普通 ref，行为稳定。

import { ref, watch } from 'vue'

const props = defineProps<{
  modelValue: boolean
  /** 有任务在跑时的文案不同 */
  busy: boolean
  downloading: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [v: boolean]
  confirm: [minimizeToTray: boolean]
  cancel: []
}>()

const minimize = ref(false)

watch(
  () => props.modelValue,
  (v) => {
    if (v) minimize.value = false
  },
)

function onConfirm() {
  emit('confirm', minimize.value)
  emit('update:modelValue', false)
}

function onCancel() {
  emit('cancel')
  emit('update:modelValue', false)
}
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    :title="busy ? (downloading ? '正在下载中' : '正在获取中') : '退出程序'"
    width="420px"
    :close-on-click-modal="false"
    :close-on-press-escape="false"
    append-to-body
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <p v-if="busy" class="text">
      正在{{ downloading ? '下载' : '获取' }}作品，关闭程序会先结束当前任务。是否继续关闭？
    </p>
    <p v-else class="text">确定要退出程序吗？</p>

    <el-checkbox v-if="!busy" v-model="minimize" class="tray-check">
      最小化到任务栏（不再提示）
    </el-checkbox>

    <template #footer>
      <el-button @click="onCancel">{{ busy ? '否，继续' : '取消' }}</el-button>
      <el-button type="primary" @click="onConfirm">
        {{ busy ? '是，结束任务并关闭' : '确定' }}
      </el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.text {
  margin: 0 0 12px;
  line-height: 1.6;
  color: #303133;
}

.tray-check {
  margin-top: 4px;
}
</style>
