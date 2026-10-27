import { createRouter, createWebHashHistory } from 'vue-router'
import DownloadView from '@/views/DownloadView.vue'

// 侧边栏多页布局（用户选择）。hash 模式：Tauri 打包后无后端路由。
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', name: 'download', component: DownloadView, meta: { title: '下载' } },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/views/SettingsView.vue'),
      meta: { title: '设置' },
    },
    {
      path: '/about',
      name: 'about',
      component: () => import('@/views/AboutView.vue'),
      meta: { title: '关于' },
    },
  ],
})

export default router
