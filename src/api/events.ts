// 事件订阅：事件名必须与 src-tauri/src/commands.rs 的常量一致。

import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  CloseRequest,
  FetchingProfile,
  CookieStatus,
  DownloadSummary,
  FetchPageEvent,
  FetchSummary,
  LogEvent,
  ProfileInfo,
  ProgressEvent,
  SpeedEvent,
} from '@/types/models'

export const EV = {
  progress: 'progress:update',
  speed: 'speed:update',
  fetchPage: 'fetch:page',
  log: 'fetch:log',
  fetchDone: 'fetch:done',
  fetchProfile: 'fetch:profile',
  downloadDone: 'download:done',
  cookie: 'cookie:status',
  profile: 'auth:profile',
  closeRequest: 'window:close-request',
  focused: 'window:focused',
} as const

export const onProgress = (cb: (e: ProgressEvent) => void): Promise<UnlistenFn> =>
  listen<ProgressEvent>(EV.progress, (ev) => cb(ev.payload))

export const onSpeed = (cb: (e: SpeedEvent) => void): Promise<UnlistenFn> =>
  listen<SpeedEvent>(EV.speed, (ev) => cb(ev.payload))

export const onFetchPage = (cb: (e: FetchPageEvent) => void): Promise<UnlistenFn> =>
  listen<FetchPageEvent>(EV.fetchPage, (ev) => cb(ev.payload))

export const onLog = (cb: (e: LogEvent) => void): Promise<UnlistenFn> =>
  listen<LogEvent>(EV.log, (ev) => cb(ev.payload))

export const onFetchDone = (
  cb: (e: FetchSummary & { error?: string; stopped?: boolean }) => void,
): Promise<UnlistenFn> =>
  listen<FetchSummary & { error?: string; stopped?: boolean }>(EV.fetchDone, (ev) =>
    cb(ev.payload),
  )

export const onDownloadDone = (cb: (e: DownloadSummary) => void): Promise<UnlistenFn> =>
  listen<DownloadSummary>(EV.downloadDone, (ev) => cb(ev.payload))

export const onCookieStatus = (cb: (e: CookieStatus) => void): Promise<UnlistenFn> =>
  listen<CookieStatus>(EV.cookie, (ev) => cb(ev.payload))

export const onProfile = (cb: (e: ProfileInfo) => void): Promise<UnlistenFn> =>
  listen<ProfileInfo>(EV.profile, (ev) => cb(ev.payload))

export const onCloseRequest = (cb: (e: CloseRequest) => void): Promise<UnlistenFn> =>
  listen<CloseRequest>(EV.closeRequest, (ev) => cb(ev.payload))

/** 窗口重新获得焦点（从托盘恢复）：用于同步后端最新状态 */
export const onWindowFocused = (cb: () => void): Promise<UnlistenFn> =>
  listen(EV.focused, () => cb())

/** 抓取一开始就拿到用户名，供状态栏显示 */
export const onFetchProfile = (cb: (e: FetchingProfile) => void): Promise<UnlistenFn> =>
  listen<FetchingProfile>(EV.fetchProfile, (ev) => cb(ev.payload))
