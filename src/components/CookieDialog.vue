<script setup lang="ts">
// 抖音登录：内嵌 WebviewWindow 扫码 → Rust 侧读 cookie 存储（sessionid 是 HttpOnly）。
// 保底通道：手动粘贴 cookie。

import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { api } from '@/api/ipc'
import type { LoginResult } from '@/types/models'

const visible = defineModel<boolean>('visible', { default: false })

const emit = defineEmits<{
  saved: [r: LoginResult]
}>()

const logging = ref(false)
const manualCookie = ref('')
const manualSaving = ref(false)

async function startQrLogin() {
  logging.value = true
  try {
    await api.openLogin()
    const res = await api.waitLogin()
    if (res) {
      if (res.status === 'ok') {
        ElMessage.success(`登录成功：${res.profile.nickname || '已获取账号'}`)
      } else {
        ElMessage.warning('Cookie 已保存，但校验未通过')
      }
      emit('saved', res)
      visible.value = false
    } else {
      ElMessage.info('登录窗口已关闭，未获取到 Cookie')
    }
  } catch (e) {
    ElMessage.error((e as Error).message)
  } finally {
    logging.value = false
  }
}

async function cancelQr() {
  await api.cancelLogin().catch(() => {})
  logging.value = false
}

async function saveManual() {
  const c = manualCookie.value.trim()
  if (c.length < 20) {
    ElMessage.warning('Cookie 看起来太短，请完整粘贴')
    return
  }
  manualSaving.value = true
  try {
    const st = await api.setCookie(c)
    if (st.status === 'ok') {
      ElMessage.success('Cookie 有效')
    } else {
      ElMessage.warning('Cookie 已保存，但校验未通过（可能已过期）')
    }
    emit('saved', { cookie: c, status: st.status, profile: st.profile })
    if (st.status === 'ok') visible.value = false
  } catch (e) {
    ElMessage.error((e as Error).message)
  } finally {
    manualSaving.value = false
  }
}
</script>

<template>
  <el-dialog
    v-model="visible"
    title="抖音登录"
    width="600px"
    :close-on-click-modal="false"
    append-to-body
  >
    <el-alert type="info" :closable="false" show-icon>
      <template #title>扫码登录（推荐）</template>
      点击下方按钮打开内嵌登录窗口，扫码并完成二次验证后，程序会自动读取 Cookie 并关闭窗口。
    </el-alert>

    <div class="qr-row">
      <el-button type="primary" :loading="logging" @click="startQrLogin">
        {{ logging ? '等待扫码中…' : '打开登录窗口' }}
      </el-button>
      <el-button v-if="logging" @click="cancelQr">取消</el-button>
    </div>

    <el-divider>或者手动粘贴 Cookie</el-divider>

    <el-input
      v-model="manualCookie"
      type="textarea"
      :rows="4"
      placeholder="从浏览器开发者工具 Network 面板复制完整 Cookie，需包含 sessionid"
      class="selectable"
    />
    <div class="manual-actions">
      <el-button :loading="manualSaving" @click="saveManual">保存并校验</el-button>
    </div>
  </el-dialog>
</template>

<style scoped>
.qr-row {
  margin: 14px 0 4px;
  display: flex;
  gap: 8px;
}

.manual-actions {
  margin-top: 10px;
  text-align: right;
}
</style>
