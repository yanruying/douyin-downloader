<script setup lang="ts">
// 设置页：两栏布局。
//
// 两栏布局：左侧为下载相关设置，右侧为交互与路径信息，
// 每栏内部按小节分组，改动后底部浮出保存条。

import { onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { open } from '@tauri-apps/plugin-dialog'
import { api } from '@/api/ipc'
import { settings } from '@/stores/app'
import type { Settings } from '@/types/models'

const form = ref<Settings | null>(null)
const saving = ref(false)
const appDir = ref('')
const downloadBase = ref('')
const dirty = ref(false)

onMounted(async () => {
  const init = await api.initApp().catch(() => null)
  if (init) {
    form.value = init.settings
    settings.value = init.settings
    appDir.value = init.appDir
    downloadBase.value = init.downloadBase
  }
})

function touch() {
  dirty.value = true
}

async function pickFolder() {
  const dir = await open({ directory: true, multiple: false, title: '选择下载根目录' })
  if (typeof dir === 'string' && form.value) {
    form.value.path = dir
    touch()
  }
}

async function save() {
  if (!form.value) return
  saving.value = true
  try {
    await api.saveSettings(form.value)
    settings.value = form.value
    dirty.value = false
    ElMessage.success('设置已保存')
  } catch (e) {
    ElMessage.error((e as Error).message)
  } finally {
    saving.value = false
  }
}

async function openDir(p: string) {
  if (p) await api.openPath(p).catch(() => {})
}



/// 撤销：重新拉后端已保存的设置，丢弃本地修改
async function revert() {
  const init = await api.initApp().catch(() => null)
  if (init) form.value = init.settings
  dirty.value = false
}

async function resetPath() {
  if (!form.value) return
  form.value.path = ''
  touch()
}
</script>

<template>
  <div v-loading="!form" class="settings">
    <template v-if="form">
      <div class="grid">
        <!-- 左栏：下载相关 -->
        <section class="col">
          <el-card shadow="never">
            <template #header>
              <div class="card-head">
                <el-icon><FolderOpened /></el-icon>
                <span>保存位置</span>
              </div>
            </template>

            <div class="field">
              <label>下载根目录</label>
              <div class="row">
                <el-input
                  v-model="form.path"
                  :placeholder="`留空 = ${downloadBase}`"
                  @input="touch"
                />
                <el-button @click="pickFolder">浏览</el-button>
                <el-button v-if="form.path" text @click="resetPath">恢复默认</el-button>
              </div>
            </div>

            <div class="field">
              <label>并发线程数</label>
              <div class="row">
                <el-slider
                  v-model="form.threads"
                  :min="1"
                  :max="8"
                  :step="1"
                  show-input
                  class="slider"
                  @change="touch"
                />
              </div>
              <p class="tip">建议 4~6，上限 8，过高易触发风控</p>
            </div>
          </el-card>

          <el-card shadow="never" class="mt">
            <template #header>
              <div class="card-head">
                <el-icon><Document /></el-icon>
                <span>文件组织</span>
              </div>
            </template>

            <div class="switch-row">
              <div>
                <div class="sw-title">合集分文件夹</div>
                <p class="tip">有合集的作品放入「合集名」子目录</p>
              </div>
              <el-switch v-model="form.useMixFolder" @change="touch" />
            </div>

            <div class="switch-row">
              <div>
                <div class="sw-title">文件名加日期前缀</div>
                <p class="tip"><code>2025-03-04_文案.mp4</code></p>
              </div>
              <el-switch v-model="form.includeDateInFilename" @change="touch" />
            </div>

            <p class="tip note-tip">改动后再次下载会移动/改名已有文件，不重新下载。</p>
          </el-card>

        </section>

        <!-- 右栏：界面与系统 -->
        <section class="col">
          <el-card shadow="never">
            <template #header>
              <div class="card-head">
                <el-icon><Setting /></el-icon>
                <span>交互</span>
              </div>
            </template>

            <div class="switch-row">
              <div>
                <div class="sw-title">获取后自动全选</div>
                <p class="tip">抓取完成后自动勾选全部</p>
              </div>
              <el-switch v-model="form.autoSelectAfterFetch" @change="touch" />
            </div>

            <div class="switch-row">
              <div>
                <div class="sw-title">关闭时最小化到任务栏</div>
                <p class="tip">关闭时直接隐藏到托盘，托盘图标可唤回</p>
              </div>
              <el-switch v-model="form.closeToTray" @change="touch" />
            </div>
          </el-card>

          <el-card shadow="never" class="mt">
            <template #header>
              <div class="card-head">
                <el-icon><Download /></el-icon>
                <span>导出</span>
              </div>
            </template>

            <div class="switch-row">
              <div>
                <div class="sw-title">导出直链时带标题</div>
                <p class="tip">每行「序号.文案 + 链接」</p>
              </div>
              <el-switch v-model="form.addTitleWhenExportUrls" @change="touch" />
            </div>
          </el-card>
          <el-card shadow="never" class="mt">
            <template #header>
              <div class="card-head">
                <el-icon><InfoFilled /></el-icon>
                <span>路径信息</span>
              </div>
            </template>

            <div class="path-row">
              <span class="path-label">配置目录</span>
              <span class="mono selectable ellipsis">{{ appDir }}</span>
              <el-button text type="primary" @click="openDir(appDir)">打开</el-button>
            </div>
            <div class="path-row">
              <span class="path-label">下载目录</span>
              <span class="mono selectable ellipsis">{{ form.path || downloadBase }}</span>
              <el-button text type="primary" @click="api.openDownloadRoot()">打开</el-button>
            </div>
          </el-card>
        </section>
      </div>

      <!-- 悬浮保存条：改动后出现，避免每次都要滚到底 -->
      <transition name="fade">
        <div v-if="dirty" class="save-bar">
          <span>有未保存的修改</span>
          <div class="spacer" />
          <el-button @click="revert">撤销</el-button>
          <el-button type="primary" :loading="saving" @click="save">保存设置</el-button>
        </div>
      </transition>
    </template>
  </div>
</template>

<style scoped>
.settings {
  height: 100%;
  overflow-y: auto;
  padding-right: 6px;
  padding-bottom: 12px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(380px, 1fr));
  gap: 12px;
  align-items: start;
}

.col {
  min-width: 0;
}

.mt {
  margin-top: 12px;
}

.card-head {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  color: #303133;
}

.spacer {
  flex: 1;
}

.field {
  margin-bottom: 18px;
}

.field > label {
  display: block;
  font-size: 13px;
  color: #606266;
  margin-bottom: 6px;
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.row > .el-input {
  flex: 1;
}

.slider {
  flex: 1;
}

.tip {
  margin: 6px 0 0;
  font-size: 12px;
  color: #909399;
  line-height: 1.6;
}

.tip code {
  background: #f5f7fa;
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 11px;
  color: #606266;
}

.mb {
  margin-bottom: 10px;
}

.switch-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px dashed #f0f2f5;
}

.switch-row:last-of-type {
  border-bottom: none;
}

.sw-title {
  font-size: 13px;
  color: #303133;
  margin-bottom: 2px;
}

/* 这个提示块同时承担「拉齐左右两栏高度」的作用：
   左栏（保存位置 + 文件组织）本来就比右栏高，把提示块做大一点，
   两栏底边就齐了，视觉上不会一高一低。 */
.note-tip {
  margin-top: 12px;
  padding: 14px 16px;
  background: #f4f7fb;
  border: 1px solid #e6eef8;
  border-radius: 6px;
  font-size: 12.5px;
  line-height: 1.7;
  color: #606266;
}

.user-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
}

.u-name {
  flex: 0 0 100px;
}

.u-sec {
  flex: 1;
  min-width: 0;
}

.u-sec :deep(.el-input__inner) {
  font-size: 11px;
}

.empty {
  font-size: 12px;
  padding: 6px 0;
}

.path-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.path-label {
  flex: 0 0 62px;
  font-size: 12px;
  color: #909399;
}

.ellipsis {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  color: #606266;
}

.save-bar {
  position: sticky;
  bottom: 0;
  margin-top: 12px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: #fff;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  box-shadow: 0 -2px 12px rgba(0, 0, 0, 0.06);
  font-size: 13px;
  color: #606266;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.18s;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
