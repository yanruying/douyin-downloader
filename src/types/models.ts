// 与 Rust 侧 serde 结构一一对应的类型定义。
// 改这里必须同步改 src-tauri/src/models.rs。

/** 四种取数模式，与 Rust 的 FetchMode 对应 */
export type FetchMode = 'single' | 'post' | 'favorite' | 'collect'
export type CookieStatus = 'empty' | 'ok' | 'expired'
export type TaskKind = 'video' | 'image' | 'live'

export interface TaskRow {
  id: string
  awemeId: string
  date: string
  desc: string
  mix: string
  kind: TaskKind
  url: string
  ext: string
  isImage: boolean
  urlHash: string
}

/**
 * 用户列表条目。
 * 只存昵称与 sec_uid：拼链接的规则是固定的，存整条 URL 反而会随
 * 粘贴内容（带 query、带分享文案）变脏。
 */
export interface UserEntry {
  username: string
  secUid: string
}

export interface Settings {
  path: string
  cookie: string
  useMixFolder: boolean
  includeDateInFilename: boolean
  autoSelectAfterFetch: boolean
  addTitleWhenExportUrls: boolean
  threads: number
  users: UserEntry[]
  nickname: string
  uniqueId: string
  avatarUrl: string
  secUid: string
  closeToTray: boolean
}

export interface ProfileInfo {
  nickname: string
  uniqueId: string
  avatarUrl: string
  secUid: string
}

export interface FetchSummary {
  nickname: string
  uniqueId: string
  /** 作品所属账号的 sec_user_id（用于目录身份标记） */
  secUid: string
  count: number
  mode: string
}

export interface ProgressEvent {
  mode: string
  completed: number
  total: number
}

export interface SpeedEvent {
  bytesPerSec: number
}

export interface FetchPageEvent {
  mode: string
  page: number
  added: number
  total: number
  tasks: TaskRow[]
}

export interface LogEvent {
  level: 'info' | 'warn' | 'error'
  text: string
}

export interface DownloadSummary {
  mode: string
  ok: number
  skipped: number
  failed: number
  stopped: boolean
  /** 失败明细：[作品描述, 失败原因] */
  failures: [string, string][]
}

/** 抓取开始后、分页之前推送的用户名 */
export interface FetchingProfile {
  mode: FetchMode
  nickname: string
  uniqueId: string
}

export interface ExportResult {
  path: string
}

export interface CookieState {
  status: CookieStatus
  profile: ProfileInfo
}

export interface LoginResult {
  cookie: string
  status: CookieStatus
  profile: ProfileInfo
}

/** Excel 导出用的精简统计（由后端解析 aweme 得到） */
export interface AwemeStat {
  awemeId: string
  desc: string
  createTime: number
  mix: string
  isAlbum: boolean
  digg: number
  comment: number
  collect: number
  share: number
  recommend: number
  duration: number
}

/** 单个页面的完整状态 */
export interface ModeState {
  mode: string
  tasks: TaskRow[]
  summary: FetchSummary
  fetching: boolean
  downloading: boolean
  progress: ProgressEvent
  lastDownload: DownloadSummary | null
  url: string
  stats: AwemeStat[]
}

export interface InitPayload {
  settings: Settings
  appDir: string
  downloadBase: string
  legacyImported: boolean
  cookieStatus: CookieStatus
  profile: ProfileInfo
  modes: ModeState[]
}

export interface StartAck {
  started: boolean
}

export interface BusyState {
  busy: boolean
  downloading: boolean
  fetching: boolean
}

export interface CloseRequest {
  busy: boolean
  downloading: boolean
}

export interface AppError {
  code: string
  message: string
}

export const KIND_LABEL: Record<TaskKind, string> = {
  video: '视频',
  image: '图片',
  live: '实况',
}

export const MODE_LABEL: Record<FetchMode, string> = {
  single: '单作品',
  post: '主页作品',
  favorite: '点赞作品',
  collect: '收藏作品',
}

/** 每个模式的输入提示 */
export const MODE_HINT: Record<FetchMode, string> = {
  single: '粘贴作品链接（支持 v.douyin.com 短链、/video/、/note/）',
  post: '粘贴主页链接；留空则获取我自己的主页作品',
  favorite: '粘贴主页链接；留空则获取我自己点赞的作品',
  collect: '只能获取到当前登录账号的收藏作品',
}

/** 该模式是否支持「留空即抓自己」 */
export const MODE_ALLOW_SELF: Record<FetchMode, boolean> = {
  single: false,
  post: true,
  favorite: true,
  collect: true,
}

/**
 * 该模式的链接输入框是否替换为只读提示。
 * 收藏接口只接受本人 sec_uid，填别人的链接必然失败。
 */
export const MODE_INPUT_READONLY: Record<FetchMode, boolean> = {
  single: false,
  post: false,
  favorite: false,
  collect: true,
}

/** 该模式是否展示「用户列表」入口 */
export const MODE_SHOW_USERLIST: Record<FetchMode, boolean> = {
  single: false,
  post: true,
  favorite: true,
  collect: false,
}

/** 获取按钮文案：主页/点赞固定「获取作品」，其余按是否留空变化 */
export const MODE_FETCH_LABEL: Record<FetchMode, string> = {
  single: '获取作品',
  post: '获取作品',
  favorite: '获取作品',
  collect: '获取我的收藏',
}

export function emptyModeState(mode: FetchMode): ModeState {
  return {
    mode,
    tasks: [],
    summary: { nickname: '', uniqueId: '', secUid: '', count: 0, mode },
    fetching: false,
    downloading: false,
    progress: { mode, completed: 0, total: 0 },
    lastDownload: null,
    url: '',
    stats: [],
  }
}
