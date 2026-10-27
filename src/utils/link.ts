// 链接提取与校验。
//
// 用户实际粘贴的内容往往不是纯链接：
//   单作品：`6.94 g@b.Nj :3pm tEu:/ 03/28 招血仆🩸# 克鲁鲁 # 慢摇 # 万圣节  https://v.douyin.com/ct0NrAHlEXA/ 复制此链接，打开Dou音搜索，直接观看视频！`
//   主页：  `长按复制此条消息，打开抖音搜索，查看TA的更多作品。 https://v.douyin.com/0Q9dvwIrhGI/`
// 所以输入后要自动抽出真正的 URL，并去掉前后噪声。

import type { FetchMode } from '@/types/models'

/** 抖音短链 */
const SHORT = /https?:\/\/v\.douyin\.com\/[A-Za-z0-9_-]+\/?/i
/** 主页链接：只取 /user/{sec_uid} 主体，丢掉 query（如 from_tab_name、vid） */
const USER = /https?:\/\/(?:www\.)?douyin\.com\/user\/[A-Za-z0-9_-]+/i
/** 作品链接：/video/{id} 或 /note/{id} */
const WORK = /https?:\/\/(?:www\.)?douyin\.com\/(?:video|note)\/\d{15,25}/i
/** 从主页面里带 modal_id 的形式 */
const MODAL = /https?:\/\/(?:www\.)?douyin\.com\/[^\s]*[?&]modal_id=\d{15,25}/i

/** 任意 http(s) 链接（兜底，用于短链之外的情况） */
const ANY_URL = /https?:\/\/[^\s\u4e00-\u9fa5，。！？、；：""''（）]+/i

/** 裸 ID：用户可能只贴了一串数字或 MS4wLjAB... */
const AWEME_ID = /\b(\d{19})\b/
const SEC_UID = /(MS4wLjAB[A-Za-z0-9_-]+)/

/**
 * 归一化：主页链接丢掉 query，作品链接只保留 /video|note/{id}，
 * 短链保持原样（保留尾部斜杠，抖音接受两种形式）。
 */
function normalize(link: string, mode: FetchMode): string {
  if (mode === 'single') {
    const work = link.match(WORK) || link.match(MODAL)
    if (work) return work[0]
    return link
  }
  const user = link.match(USER)
  if (user) return user[0]
  return link
}

/**
 * 从一段混合文本里抽出可用的链接。
 * 返回空串表示没找到；调用方据此给提示，而不是把整段噪声发给后端。
 */
export function extractLink(raw: string, mode: FetchMode): string {
  const text = raw.trim()
  if (!text) return ''

  // 已经是单个链接：仍然要归一化（去掉 query 与尾随标点），
  // 否则 /user/xxx?from_tab_name=… 这种会带着无用的参数进后端
  const cleaned = text.replace(/[，。！？、；：""''）】]+$/g, '')
  if (/^https?:\/\/\S+$/i.test(cleaned)) {
    return normalize(cleaned, mode)
  }

  // 单作品：优先作品页，其次短链（短链需要后端跟随跳转）
  if (mode === 'single') {
    const work = text.match(WORK) || text.match(MODAL)
    if (work) return work[0]
    const short = text.match(SHORT)
    if (short) return short[0]
    // 纯数字 ID 也接受，拼成标准作品链接
    const id = text.match(AWEME_ID)
    if (id) return `https://www.douyin.com/video/${id[1]}`
    const any = text.match(ANY_URL)
    return any ? any[0] : ''
  }

  // 主页类：优先 user 主页，其次短链
  const user = text.match(USER)
  if (user) return user[0]
  const short = text.match(SHORT)
  if (short) return short[0]
  const sec = text.match(SEC_UID)
  if (sec) return `https://www.douyin.com/user/${sec[1]}`
  const any = text.match(ANY_URL)
  return any ? any[0] : ''
}

/**
 * 判断抽出来的链接与当前模式是否匹配，用于给更准确的提示。
 * 返回 null 表示没问题，否则返回错误文案。
 */
export function validateLink(link: string, mode: FetchMode): string | null {
  if (!link) return null // 允许留空的模式由调用方处理
  const isShort = /v\.douyin\.com/i.test(link)

  if (mode === 'single') {
    if (isShort) return null // 短链交给后端跟随
    if (!/\/(video|note)\/\d{15,25}/i.test(link) && !/[?&]modal_id=\d{15,25}/i.test(link)) {
      return '这不像是作品链接，请粘贴 /video/ 或 /note/ 开头的地址'
    }
    return null
  }

  if (isShort) return null
  if (!/douyin\.com\/user\//i.test(link)) {
    return '这不像主页链接，请粘贴 /user/ 开头的地址'
  }
  return null
}
