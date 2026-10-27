//! 抖音相关逻辑的唯一入口（高风险隔离区）。
//!
//! 命令层只允许调用这里暴露的少数函数，不得出现任何 URL、字段名或签名调用
//! —— 这样抖音改版时改动面锁定在本目录。

pub mod api;
pub mod model;
pub mod parser;
pub mod sign;

use std::sync::Arc;

use serde_json::Value;

use crate::douyin::sign::{AbogusProvider, SignatureProvider};
use crate::error::{AppError, AppResult};
use crate::models::{CookieStatus, FetchMode, ProfileInfo};

#[derive(Clone)]
pub struct DouyinClient {
    pub client: reqwest::Client,
    pub sim: Arc<dyn SignatureProvider>,
}

impl DouyinClient {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            client: api::client()?,
            sim: Arc::new(AbogusProvider::new(Some(api::UA.to_string()))),
        })
    }

    pub async fn check_cookie(&self, cookie: &str) -> CookieStatus {
        if cookie.len() < 50 || !cookie.contains("sessionid") {
            return CookieStatus::Empty;
        }
        if api::self_profile_ok(&self.client, cookie).await {
            CookieStatus::Ok
        } else {
            CookieStatus::Expired
        }
    }

    /// 按 sec_uid 查任意账号的资料。
    /// 用户列表添加链接时用它拿到最新昵称，免得用户手填一个过期的名字。
    pub async fn profile_by_sec_uid(&self, sec_uid: &str, cookie: &str) -> AppResult<ProfileInfo> {
        api::fetch_profile_info(&self.client, sec_uid, cookie).await
    }

    /// 取当前登录账号信息（昵称 / unique_id / 头像 / sec_uid）。
    pub async fn self_profile(&self, cookie: &str) -> AppResult<ProfileInfo> {
        api::fetch_self_profile(&self.client, cookie).await
    }

    /// 抓取作品。`on_page` 每页回调一次，携带该页解析结果，前端可边抓边显示。
    ///
    /// `url` 为空时抓取**自己账号**——主页/点赞/收藏三种模式都支持，
    /// 因为「没填链接」的自然语义就是「看我的」。单作品模式必须给链接，
    /// 没有「自己的某个作品」这种默认。
    pub async fn fetch_all<F, S, P>(
        &self,
        url: &str,
        cookie: &str,
        mode: FetchMode,
        should_stop: S,
        on_profile: P,
        mut on_page: F,
    ) -> AppResult<(ProfileInfo, Vec<Value>)>
    where
        F: FnMut(u32, &[Value]),
        S: Fn() -> bool,
        P: FnOnce(&ProfileInfo),
    {
        let url = url.trim();

        // 单作品模式：直接按 aweme_id 取详情，不需要 sec_user_id
        if mode == FetchMode::Single {
            // 短链不含 aweme_id，需先跟随跳转再解析。
            let aweme_id = match api::extract_aweme_id(url) {
                Some(id) => id,
                None => api::resolve_aweme_id(&self.client, url, cookie)
                    .await
                    .ok_or_else(|| {
                        AppError::msg(
                            "没能从链接里识别出作品 ID。请粘贴作品页地址（/video/ 或 /note/），或抖音分享的短链",
                        )
                    })?,
            };
            let detail = api::fetch_aweme_detail(&self.client, &self.sim, &aweme_id, cookie).await?;
            let profile = api::profile_from_aweme(&detail);
            on_profile(&profile);
            on_page(1, std::slice::from_ref(&detail));
            return Ok((profile, vec![detail]));
        }

        // 收藏接口仅支持当前登录账号（他人 sec_uid 一律返回 3002279，
        // 与风控无关：同账号主页作品可正常抓到，冷却后重试结果不变）。
        // 与其让用户看到一串风控码，不如直接说清楚限制。
        let (sec, self_profile) = if url.is_empty() {
            let p = api::fetch_self_profile(&self.client, cookie).await?;
            if p.sec_uid.is_empty() {
                return Err(AppError::msg("无法获取本人账号标识，请重新登录后再试"));
            }
            (p.sec_uid.clone(), Some(p))
        } else {
            let sec = api::resolve_sec_user_id(&self.client, url, cookie)
                .await
                .ok_or_else(|| AppError::msg("无法从链接解析 sec_user_id"))?;

            if mode == FetchMode::Collect {
                // 只在这一模式下校验，避免多打一次 profile 请求
                let me = api::fetch_self_profile(&self.client, cookie).await?;
                if !me.sec_uid.is_empty() && me.sec_uid != sec {
                    return Err(AppError::msg(
                        "收藏作品只能获取当前登录账号的收藏。请留空链接直接获取，或先登录该账号",
                    ));
                }
            }
            (sec, None)
        };

        let profile = match self_profile {
            Some(p) => p,
            None => api::fetch_profile_info(&self.client, &sec, cookie).await?,
        };
        // 网络开始分页之前就知道昵称了：立刻推给前端，
        // 这样抓取过程中底部状态栏就能显示「正在获取 某某 的作品」
        on_profile(&profile);

        let ctx = api::FetchCtx {
            client: self.client.clone(),
            sim: self.sim.clone(),
            cookie: cookie.to_string(),
        };
        let list = ctx.fetch_all(&sec, mode, should_stop, on_page).await?;
        Ok((profile, list))
    }

    /// 一次性拿全部作品（不含增量回调）。
    pub async fn fetch_plain(
        &self,
        url: &str,
        cookie: &str,
        mode: FetchMode,
    ) -> AppResult<(ProfileInfo, Vec<Value>)> {
        self.fetch_all(url, cookie, mode, || false, |_| {}, |_, _| {}).await
    }

    /// 本人主页链接（设置页展示 / 调试用）。
    pub async fn self_home_url(&self, cookie: &str) -> AppResult<String> {
        let p = api::fetch_self_profile(&self.client, cookie).await?;
        if p.sec_uid.is_empty() {
            return Err(AppError::msg("无法获取本人账号标识"));
        }
        Ok(format!("https://www.douyin.com/user/{}", p.sec_uid))
    }
}

impl Default for DouyinClient {
    fn default() -> Self {
        Self::new().expect("failed to build douyin client")
    }
}
