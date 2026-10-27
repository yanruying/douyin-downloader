// invoke 薄封装：一个命令一个函数，不做业务判断。

import { invoke } from '@tauri-apps/api/core'
import type {
  AppError,
  ProfileInfo,
  BusyState,
  CookieState,
  DownloadSummary,
  ExportResult,
  FetchMode,
  InitPayload,
  LoginResult,
  ModeState,
  Settings,
  StartAck,
  TaskRow,
} from '@/types/models'

export class IpcError extends Error {
  code: string
  constructor(e: unknown) {
    const err = e as Partial<AppError>
    super(err?.message ?? String(e))
    this.code = err?.code ?? 'UNKNOWN'
    this.name = 'IpcError'
  }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args)
  } catch (e) {
    throw new IpcError(e)
  }
}

export const api = {
  initApp: () => call<InitPayload>('init_app'),

  getSettings: () => call<Settings>('get_settings'),
  saveSettings: (settings: Settings) => call<void>('save_settings', { settings }),

  openLogin: () => call<void>('open_login'),
  waitLogin: () => call<LoginResult | null>('wait_login'),
  cancelLogin: () => call<void>('cancel_login'),
  checkCookie: () => call<CookieState>('check_cookie'),
  setCookie: (cookie: string) => call<CookieState>('set_cookie', { cookie }),
  logout: () => call<void>('logout'),
  lookupProfile: (secUid: string) => call<ProfileInfo>('lookup_profile', { secUid }),

  startFetch: (url: string, mode: FetchMode) =>
    call<StartAck>('start_fetch', { req: { url, mode } }),
  batchFetch: (urls: string[], mode: FetchMode) =>
    call<StartAck>('batch_fetch', { req: { urls, mode } }),
  stopFetch: (mode: FetchMode) => call<void>('stop_fetch', { mode }),

  startDownload: (mode: FetchMode, ids: string[]) =>
    call<BusyState>('start_download', { req: { mode, ids } }),
  stopDownload: (mode: FetchMode) => call<void>('stop_download', { mode }),

  queryBusy: () => call<BusyState>('query_busy'),
  stopAll: () => call<void>('stop_all'),

  getRuntime: () => call<ModeState[]>('get_runtime'),
  getTasks: (mode: FetchMode) => call<TaskRow[]>('get_tasks', { mode }),
  clearTasks: (mode: FetchMode) => call<void>('clear_tasks', { mode }),

  exportUrls: (mode: FetchMode) => call<ExportResult>('export_urls', { mode }),
  exportExcel: (mode: FetchMode) => call<ExportResult>('export_excel', { mode }),

  appVersion: () => call<string>('app_version'),
  downloadBase: () => call<string>('download_base'),
  openDownloadRoot: () => call<void>('open_download_root'),
  revealPath: (path: string) => call<void>('reveal_path', { path }),
  openPath: (path: string) => call<void>('open_path', { path }),
  debugCookieProbe: () => call<[number, boolean, string[]]>('debug_cookie_probe'),
}

export type { DownloadSummary }
