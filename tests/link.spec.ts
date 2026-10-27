// 链接提取的回归测试。
//
// 用例全部来自用户实际粘贴的内容 —— 抖音分享出来的是「文案 + 短链 + 提示语」，
// 直接当 URL 用会解析失败。

import { describe, expect, it } from 'vitest'
import { extractLink, validateLink } from '@/utils/link'

describe('extractLink - 单作品', () => {
  // 用户提供的真实分享文本
  const share =
    '6.94 g@b.Nj :3pm tEu:/ 03/28 招血仆🩸# 克鲁鲁 # 慢摇 # 万圣节  https://v.douyin.com/ct0NrAHlEXA/ 复制此链接，打开Dou音搜索，直接观看视频！'

  it('从整段分享文案里抽出短链', () => {
    expect(extractLink(share, 'single')).toBe('https://v.douyin.com/ct0NrAHlEXA/')
  })

  it('识别 /video/ 链接', () => {
    const s = '视频：https://www.douyin.com/video/7700914601168757733，看这个'
    expect(extractLink(s, 'single')).toBe('https://www.douyin.com/video/7700914601168757733')
  })

  it('识别 /note/（图集）链接', () => {
    const s = '图集：https://www.douyin.com/note/7694506535757577061'
    expect(extractLink(s, 'single')).toBe('https://www.douyin.com/note/7694506535757577061')
  })

  it('裸 aweme_id 拼成标准链接', () => {
    expect(extractLink('7700914601168757733', 'single')).toBe(
      'https://www.douyin.com/video/7700914601168757733',
    )
  })

  it('纯链接原样返回（去掉尾部中文标点）', () => {
    expect(extractLink('https://www.douyin.com/video/7700914601168757733，', 'single')).toBe(
      'https://www.douyin.com/video/7700914601168757733',
    )
  })

  it('识别 modal_id 形式', () => {
    const s = 'https://www.douyin.com/user/MS4wLjABxxxx?modal_id=7700914601168757735'
    expect(extractLink(s, 'single')).toBe(s)
  })

  it('没有链接时返回空串', () => {
    expect(extractLink('随便一句话', 'single')).toBe('')
    expect(extractLink('', 'single')).toBe('')
  })
})

describe('extractLink - 主页 / 点赞', () => {
  const share =
    '长按复制此条消息，打开抖音搜索，查看TA的更多作品。 https://v.douyin.com/0Q9dvwIrhGI/'

  it('从分享文案里抽出短链', () => {
    expect(extractLink(share, 'post')).toBe('https://v.douyin.com/0Q9dvwIrhGI/')
  })

  it('带 query 的主页链接保留主体、去掉 query', () => {
    const s =
      'https://www.douyin.com/user/MS4wLjABAAAAqor8exxqvpK4gZSBdZDeFmnYX4sBxUtfTXygVNN7AaM?from_tab_name=main&vid=7700914601168757733'
    const got = extractLink(s, 'post')
    expect(got).toBe(
      'https://www.douyin.com/user/MS4wLjABAAAAqor8exxqvpK4gZSBdZDeFmnYX4sBxUtfTXygVNN7AaM',
    )
  })

  it('裸 sec_uid 拼成主页链接', () => {
    expect(extractLink('MS4wLjABAAAAqor8exxqvpK4gZSBdZDeFmnYX4sBxUtfTXygVNN7AaM', 'post')).toBe(
      'https://www.douyin.com/user/MS4wLjABAAAAqor8exxqvpK4gZSBdZDeFmnYX4sBxUtfTXygVNN7AaM',
    )
  })

  it('点赞模式与主页模式规则一致', () => {
    expect(extractLink(share, 'favorite')).toBe('https://v.douyin.com/0Q9dvwIrhGI/')
  })
})

describe('validateLink', () => {
  it('单作品模式拒绝主页链接', () => {
    expect(validateLink('https://www.douyin.com/user/MS4wLjABxxx', 'single')).toContain(
      '作品链接',
    )
  })

  it('主页模式拒绝作品链接', () => {
    expect(
      validateLink('https://www.douyin.com/video/7700914601168757733', 'post'),
    ).toContain('主页链接')
  })

  it('短链在两种模式下都放行（交给后端跟随跳转）', () => {
    expect(validateLink('https://v.douyin.com/abc/', 'single')).toBeNull()
    expect(validateLink('https://v.douyin.com/abc/', 'post')).toBeNull()
  })

  it('空串交给调用方处理', () => {
    expect(validateLink('', 'post')).toBeNull()
  })
})
