//! 抖音 HTTP 接口层：参数构造、签名调用、分页抓取与错误码处理。
//!
//! 硬编码的指纹参数全部收在 `RequestProfile`，抖音改版时只改这一处。

use std::sync::Arc;
use std::time::{Duration, Instant};

use regex::Regex;
use reqwest::Client;
use serde_json::Value;

use crate::douyin::model::{Aweme, PostData, ProfileData, ProfileUser};
use crate::douyin::sign::SignatureProvider;
use crate::error::{AppError, AppResult};
use crate::models::{FetchMode, ProfileInfo};

pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0";
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
pub const PAGE_COUNT: u32 = 50;

/// 收藏接口的硬上限：超过 20 会被服务端拒绝（status_code=5）。
pub const COLLECT_PAGE_COUNT: u32 = 20;

/// 单个模式最多翻多少页，防止服务端游标异常时无限翻页。
/// 50 条/页 × 2000 页 = 10 万条，远超正常账号规模。
pub const MAX_PAGES: u32 = 2000;

/// 同一页遇到风控时最多重试几次。
///
/// 几千条的大列表至少要翻几十页，中途任意一页踩到限流就会丢掉后面全部内容。
/// 抖音的限流码（2150/3002279）通常一两分钟自行解除，所以退避要够长；
/// 同时设上限，避免用户面对一个永久转圈的界面。
pub const MAX_PAGE_RETRIES: u32 = 5;

/// 第 n 次风控重试的等待时长：5s、10s、20s、40s、60s。
fn retry_backoff(attempt: u32) -> Duration {
    let secs = 5u64.saturating_mul(1u64 << (attempt.saturating_sub(1).min(4)));
    Duration::from_secs(secs.min(60))
}

/// 所有会被抖音风控盯上的固定参数。
pub struct RequestProfile;

impl RequestProfile {
    pub fn post_params(
        sec_user_id: &str,
        max_cursor: i64,
        count: u32,
        first_page: bool,
    ) -> Vec<(String, String)> {
        let mut v = Self::list_base(sec_user_id, max_cursor, count);
        v.push(("need_time_list".into(), if first_page { "1" } else { "0" }.into()));
        v.push(("time_list_query".into(), "0".into()));
        v.extend(Self::common_params());
        v
    }

    pub fn favorite_params(sec_user_id: &str, max_cursor: i64, count: u32) -> Vec<(String, String)> {
        let mut v = Self::list_base(sec_user_id, max_cursor, count);
        v.push(("need_time_list".into(), "0".into()));
        v.push(("time_list_query".into(), "0".into()));
        v.extend(Self::common_params());
        v
    }

    /// 收藏列表：与其他三个接口形态不同——它是 **POST**，
    /// 而且分页参数 `count/cursor` 要放在 **body** 里，query 只带通用指纹。
    /// 接口路径为 `aweme/v1/web/aweme/listcollection/`。
    pub fn collect_query(sec_user_id: &str) -> Vec<(String, String)> {
        let mut v = vec![
            ("device_platform".into(), "webapp".into()),
            ("aid".into(), "6383".into()),
            ("channel".into(), "channel_pc_web".into()),
            ("sec_user_id".into(), sec_user_id.into()),
        ];
        v.extend(Self::common_params());
        v
    }

    /// 收藏接口的 body：`count` 与 `cursor` 都放这里。
    ///
    /// **count 上限是 20**。传 50 时接口返回 `status_code=5` 且不带任何数据
    /// 超出会被服务端拒绝并返回空数据。主页与点赞接口上限为 50。
    pub fn collect_body(count: u32, cursor: i64) -> String {
        let n = count.clamp(1, COLLECT_PAGE_COUNT);
        format!("count={n}&cursor={cursor}")
    }

    fn list_base(sec_user_id: &str, max_cursor: i64, count: u32) -> Vec<(String, String)> {
        vec![
            ("device_platform".into(), "webapp".into()),
            ("aid".into(), "6383".into()),
            ("channel".into(), "channel_pc_web".into()),
            ("sec_user_id".into(), sec_user_id.into()),
            ("max_cursor".into(), max_cursor.to_string()),
            ("count".into(), count.to_string()),
            ("locate_query".into(), "false".into()),
            ("show_live_replay_strategy".into(), "1".into()),
            ("whale_cut_token".into(), "".into()),
            ("cut_version".into(), "1".into()),
            ("publish_video_strategy_type".into(), "2".into()),
            ("from_user_page".into(), "1".into()),
        ]
    }

    /// 单作品详情：aweme_id + 通用指纹。
    pub fn detail_params(aweme_id: &str) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = vec![
            ("device_platform".into(), "webapp".into()),
            ("aid".into(), "6383".into()),
            ("channel".into(), "channel_pc_web".into()),
            ("aweme_id".into(), aweme_id.into()),
            ("locate_query".into(), "false".into()),
            ("from_user_page".into(), "1".into()),
        ];
        v.extend(Self::common_params());
        v
    }

    fn common_params() -> Vec<(String, String)> {
        vec![
            ("update_version_code".into(), "170400".into()),
            ("pc_client_type".into(), "1".into()),
            ("pc_libra_divert".into(), "Windows".into()),
            ("support_h265".into(), "1".into()),
            ("support_dash".into(), "0".into()),
            ("version_code".into(), "290100".into()),
            ("version_name".into(), "29.1.0".into()),
            ("cookie_enabled".into(), "true".into()),
            ("screen_width".into(), "1920".into()),
            ("screen_height".into(), "1080".into()),
            ("browser_language".into(), "zh-CN".into()),
            ("browser_platform".into(), "Win32".into()),
            ("browser_name".into(), "Edge".into()),
            ("browser_version".into(), "131.0.0.0".into()),
            ("browser_online".into(), "true".into()),
            ("engine_name".into(), "Blink".into()),
            ("engine_version".into(), "131.0.0.0".into()),
            ("os_name".into(), "Windows".into()),
            ("os_version".into(), "10".into()),
            ("cpu_core_num".into(), "12".into()),
            ("device_memory".into(), "8".into()),
            ("platform".into(), "PC".into()),
            ("downlink".into(), "10".into()),
            ("effective_type".into(), "4g".into()),
            ("round_trip_time".into(), "50".into()),
        ]
    }
}

/// 参数编码遵循 `quote_plus` 规则：保留 `A-Za-z0-9` 与 `_.-~`，空格转为 `+`。
/// 注意 `_` 必须原样输出，编码成 `%5F` 会导致签名不匹配。
fn py_quote(s: &str, out: &mut String) {
    const SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_.-~";
    for b in s.as_bytes() {
        if SAFE.contains(b) {
            out.push(*b as char);
        } else if *b == b' ' {
            out.push('+');
        } else {
            out.push('%');
            out.push_str(&format!("{b:02X}"));
        }
    }
}

pub fn urlencode(params: &[(String, String)]) -> String {
    let mut s = String::new();
    for (i, (k, v)) in params.iter().enumerate() {
        if i > 0 {
            s.push('&');
        }
        py_quote(k, &mut s);
        s.push('=');
        py_quote(v, &mut s);
    }
    s
}

pub fn client() -> AppResult<Client> {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(UA)
        .pool_max_idle_per_host(16)
        .build()
        .map_err(AppError::from)
}

/// 从任意抖音链接提取 `sec_user_id`。
pub fn extract_sec_user_id(input: &str) -> Option<String> {
    let patterns = [
        r"/user/([A-Za-z0-9_.\-]+)",
        r"sec_user_id=([A-Za-z0-9_.\-]+)",
        r"(MS4wLjAB[A-Za-z0-9_\-]+)",
    ];
    for p in patterns {
        if let Ok(re) = Regex::new(p) {
            if let Some(c) = re.captures(input) {
                if let Some(m) = c.get(1) {
                    return Some(m.as_str().to_string());
                }
            }
        }
    }
    None
}

/// 从作品链接提取 aweme_id。
/// 支持 `/video/7300…`、`/note/7300…`、`modal_id=7300…`、`aweme_id=7300…`，
/// 也支持 `v.douyin.com/xxxx` 短链（由调用方先跟随跳转）。
pub fn extract_aweme_id(input: &str) -> Option<String> {
    let patterns = [
        r"/video/(\d{15,25})",
        r"/note/(\d{15,25})",
        r"modal_id=(\d{15,25})",
        r"aweme_id=(\d{15,25})",
        r"/(\d{19})",
    ];
    for p in patterns {
        if let Ok(re) = Regex::new(p) {
            if let Some(c) = re.captures(input) {
                if let Some(m) = c.get(1) {
                    return Some(m.as_str().to_string());
                }
            }
        }
    }
    None
}

/// 短链跟随后再提取（对应 `resolve_short_url_and_extract`）。
pub async fn resolve_sec_user_id(client: &Client, url: &str, cookie: &str) -> Option<String> {
    if let Some(sec) = extract_sec_user_id(url) {
        if url.contains("/user/") || url.contains("MS4wLjAB") {
            return Some(sec);
        }
    }
    if let Ok(r) = client
        .get(url)
        .header("Cookie", cookie)
        .header("Referer", "https://www.douyin.com/")
        .send()
        .await
    {
        if let Some(sec) = extract_sec_user_id(r.url().as_str()) {
            return Some(sec);
        }
    }
    extract_sec_user_id(url)
}

/// 短链跟随后提取 aweme_id。
pub async fn resolve_aweme_id(client: &Client, url: &str, cookie: &str) -> Option<String> {
    if let Some(id) = extract_aweme_id(url) {
        return Some(id);
    }
    if let Ok(r) = client
        .get(url)
        .header("Cookie", cookie)
        .header("Referer", "https://www.douyin.com/")
        .send()
        .await
    {
        return extract_aweme_id(r.url().as_str());
    }
    None
}

async fn get_text(client: &Client, url: &str, cookie: &str, referer: &str) -> AppResult<String> {
    let mut last: Option<AppError> = None;
    let mut delay = Duration::from_secs(1);
    for attempt in 0..=3u32 {
        let r = client
            .get(url)
            .header("Cookie", cookie)
            .header("Referer", referer)
            .header("x-tt-argus", "1")
            .send()
            .await;
        match r {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 429 || status.as_u16() == 503 {
                    last = Some(AppError::msg(format!("接口限流 HTTP {status}")));
                } else if !status.is_success() {
                    return Err(AppError::msg(format!("接口返回 HTTP {status}")));
                } else {
                    return resp.text().await.map_err(AppError::from);
                }
            }
            Err(e) => {
                last = Some(AppError::msg(format!("请求失败: {e}")));
            }
        }
        if attempt < 3 {
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(Duration::from_secs(30));
        }
    }
    Err(last.unwrap_or_else(|| AppError::msg("请求失败")))
}

fn profile_of(u: ProfileUser) -> ProfileInfo {
    // 先算头像再解构：best_avatar 借用 self，放在 move 之前
    let avatar = u.best_avatar();
    ProfileInfo {
        nickname: u.nickname.unwrap_or_default(),
        unique_id: u.unique_id.unwrap_or_default(),
        avatar_url: avatar,
        sec_uid: u.sec_uid.unwrap_or_default(),
    }
}

/// 用户资料（他人主页）。
pub async fn fetch_profile_info(
    client: &Client,
    sec_user_id: &str,
    cookie: &str,
) -> AppResult<ProfileInfo> {
    let url = format!(
        "https://www.douyin.com/aweme/v1/web/user/profile/other/?device_platform=webapp&aid=6383&channel=channel_pc_web&sec_user_id={sec_user_id}&from_user_page=1"
    );
    let text = get_text(client, &url, cookie, "https://www.douyin.com/").await?;
    let resp: crate::douyin::model::ApiResponse<ProfileData> = serde_json::from_str(&text)
        .map_err(|e| AppError::msg(format!("用户信息解析失败: {e}")))?;
    if resp.status_code != 0 {
        return Err(AppError::msg(format!(
            "获取用户信息失败: status_code={}, message={}",
            resp.status_code,
            resp.status_msg.unwrap_or_default()
        )));
    }
    resp.data
        .user
        .map(profile_of)
        .ok_or_else(|| AppError::msg("用户信息为空，Cookie 可能已过期"))
}

/// 登录账号自己的资料。
pub async fn fetch_self_profile(client: &Client, cookie: &str) -> AppResult<ProfileInfo> {
    let url = "https://www.douyin.com/aweme/v1/web/user/profile/self/?device_platform=webapp&aid=6383&channel=channel_pc_web";
    let text = get_text(client, url, cookie, "https://www.douyin.com/").await?;
    let resp: crate::douyin::model::ApiResponse<ProfileData> = serde_json::from_str(&text)
        .map_err(|e| AppError::msg(format!("账号信息解析失败: {e}")))?;
    if resp.status_code != 0 {
        return Err(AppError::msg(format!(
            "获取账号信息失败: status_code={}",
            resp.status_code
        )));
    }
    resp.data
        .user
        .map(profile_of)
        .ok_or_else(|| AppError::msg("账号信息为空，Cookie 可能已过期"))
}

/// 登录态自检。
pub async fn self_profile_ok(client: &Client, cookie: &str) -> bool {
    matches!(fetch_self_profile(client, cookie).await, Ok(_))
}

/// 单作品详情。
pub async fn fetch_aweme_detail(
    client: &Client,
    sim: &Arc<dyn SignatureProvider>,
    aweme_id: &str,
    cookie: &str,
) -> AppResult<Value> {
    let qs = urlencode(&RequestProfile::detail_params(aweme_id));
    let (signed, _) = sim.sign(&qs, "");
    let url = format!("https://www.douyin.com/aweme/v1/web/aweme/detail/?{signed}");
    let text = get_text(client, &url, cookie, "https://www.douyin.com/").await?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| AppError::msg(format!("作品详情解析失败: {e}")))?;
    let code = root.get("status_code").and_then(|v| v.as_i64()).unwrap_or(0);
    if code != 0 {
        return Err(AppError::msg(format!(
            "作品详情接口错误: status_code={code}, message={}",
            root.get("status_msg").and_then(|v| v.as_str()).unwrap_or("")
        )));
    }
    root.get("aweme_detail")
        .cloned()
        .filter(|v| !v.is_null())
        .ok_or_else(|| AppError::msg("作品不存在或已被删除"))
}

/// 单作品模式下，从 aweme 自身取作者信息作为「用户」。
pub fn profile_from_aweme(detail: &Value) -> ProfileInfo {
    let Ok(a) = serde_json::from_value::<Aweme>(detail.clone()) else {
        return ProfileInfo::default();
    };
    a.author
        .map(|au| {
            let avatar = au.best_avatar();
            ProfileInfo {
                nickname: au.nickname.unwrap_or_default(),
                unique_id: au.unique_id.unwrap_or_default(),
                avatar_url: avatar,
                sec_uid: au.sec_uid.unwrap_or_default(),
            }
        })
        .unwrap_or_default()
}

/// 分页抓取。每页通过 `on_page` 回调增量交付。
pub struct FetchCtx {
    pub client: Client,
    pub sim: Arc<dyn SignatureProvider>,
    pub cookie: String,
    /// 覆盖接口域名。仅供测试把请求指向本地 mock 服务。
    #[doc(hidden)]
    pub base_override: Option<String>,
}

impl FetchCtx {
    /// 生产路径的构造器。
    pub fn new(client: Client, sim: Arc<dyn SignatureProvider>, cookie: String) -> Self {
        Self {
            client,
            sim,
            cookie,
            base_override: None,
        }
    }

    fn endpoint(&self, path: &str) -> String {
        match &self.base_override {
            Some(base) => format!("{base}{path}"),
            None => format!("https://www.douyin.com{path}"),
        }
    }
}

/// 分页结果。
///
/// 不再携带原始 aweme JSON：几千条作品的原始响应（每条含多套 bit_rate 与
/// url_list）会占到几十上百 MB，而调用方只需要解析后的扁平结果。每页解析完
/// 就把原始 JSON 丢掉，抓取期间内存占用保持平稳。
#[derive(Debug)]
pub struct FetchOutcome {
    /// 抓到的原始 aweme 条数
    pub count: u32,
    pub warning: Option<String>,
    /// 用户主动停止
    pub stopped: bool,
}

impl FetchOutcome {
    pub fn done(count: u32) -> Self {
        Self {
            count,
            warning: None,
            stopped: false,
        }
    }
}

impl FetchCtx {
    pub async fn fetch_all<F, S>(
        &self,
        sec_user_id: &str,
        mode: FetchMode,
        mut should_stop: S,
        mut on_page: F,
    ) -> AppResult<FetchOutcome>
    where
        F: FnMut(u32, &[Value]),
        S: FnMut() -> bool,
    {
        // 收藏是 POST + body 分页，单独走一条路径
        if mode == FetchMode::Collect {
            return self.fetch_collect(sec_user_id, should_stop, on_page).await;
        }

        let mut count = 0u32;
        let mut page = 1u32;
        let mut max_cursor = 0i64;
        // 每页最多重试一次的计数器（风控退避用）
        let mut page_retry = 0u32;
        // 中途失败的说明；非空表示「没抓全但不是全丢」
        let mut warning: Option<String> = None;
        let mut stopped = false;
        // 去掉 has_more 之后，用「作品 id 是否全是新的」判断是否到底
        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut repeat_pages = 0u32;
        // 连续空页计数：单页空不终止
        let mut empty_pages = 0u32;
        // 记录走过的游标值：服务端在列表尾部会让游标归零重放，
        // 只靠「不前进」拦不住，必须认出「这个游标我已经来过」。
        let mut visited_cursors: std::collections::HashSet<i64> =
            std::collections::HashSet::from([0i64]);

        let (path, params_fn): (&str, fn(&str, i64, u32) -> Vec<(String, String)>) = match mode {
            FetchMode::Favorite => (
                "/aweme/v1/web/aweme/favorite/",
                RequestProfile::favorite_params,
            ),
            _ => (
                "/aweme/v1/web/aweme/post/",
                |sec: &str, cur: i64, cnt: u32| RequestProfile::post_params(sec, cur, cnt, cur == 0),
            ),
        };
        let base = self.endpoint(path);

        loop {
            if should_stop() {
                stopped = true;
                break;
            }
            let params = params_fn(sec_user_id, max_cursor, PAGE_COUNT);
            let qs = urlencode(&params);
            let (signed, _) = self.sim.sign(&qs, "");
            let url = format!("{base}?{signed}");

            let started = Instant::now();

            // 单页失败不再整体中止：已经到手的页比一个错误码有价值得多。
            // 抓到一半遇到风控/网络抖动时，用户至少能下载已有部分。
            let page_result = async {
                let text =
                    get_text(&self.client, &url, &self.cookie, "https://www.douyin.com/").await?;
                let resp: crate::douyin::model::ApiResponse<PostData> =
                    serde_json::from_str(&text).map_err(|e| {
                        AppError::msg(format!("第 {page} 页解析失败: {e}（Cookie 或签名可能失效）"))
                    })?;
                Ok::<_, AppError>(resp)
            }
            .await;

            let resp = match page_result {
                Ok(r) => r,
                Err(e) => {
                    if count == 0 {
                        // 第一页就失败：没有可保留的数据，如实报错。
                        // 这里不退避重试——cookie 失效/签名失效都要用户自己处理，
                        // 让界面多转十几秒再报同样的错只会更难排查。
                        return Err(e);
                    }
                    // 中途失败多半是响应被截断或限流，重试同一页通常能过去。
                    if page_retry < MAX_PAGE_RETRIES {
                        page_retry += 1;
                        tokio::time::sleep(retry_backoff(page_retry)).await;
                        continue;
                    }
                    warning = Some(format!("第 {page} 页中断（{e}），已保留前 {}", page - 1));
                    break;
                }
            };

            if resp.status_code != 0 {
                let msg = resp.status_msg.clone().unwrap_or_default();
                let hint = describe_code(resp.status_code);
                // 风控类错误：退避后重试同一页，成功就当无事发生。
                // 直接报错只会让用户狂点，反而拖长风控时间；而只重试一次
                // 在几千条的大列表上不够用——中途任意一页踩到限流就会丢掉后面全部。
                if is_rate_limited(resp.status_code) && page_retry < MAX_PAGE_RETRIES {
                    page_retry += 1;
                    tokio::time::sleep(retry_backoff(page_retry)).await;
                    continue;
                }
                let detail = if msg.is_empty() {
                    hint
                } else {
                    format!("{hint}（{msg}）")
                };
                if count == 0 {
                    return Err(AppError::msg(format!("第 {page} 页{detail}")));
                }
                warning = Some(format!("第 {page} 页中断（{detail}），已保留前 {}", page - 1));
                break;
            }
            page_retry = 0;

            let list = resp.data.aweme_list.unwrap_or_default();
            // 空页不等于到底。
            //
            // 服务端在分片边界、限流、AB 桶切换时都可能吐一次空页，随后继续
            // 返回数据。把空页当终止条件会让几千条的作品只落下一百多条。
            // 空页只表示「这一页没东西」，继续翻即可；真正的到底由游标判定
            // （不前进 / 归零后重来 / 兜底重复计数）。
            let empty_page = list.is_empty();
            // 既然不靠 has_more 收尾，就得自己判断「服务端在重复喂同一批」：
            // 游标一路前进但作品 id 全是见过的，说明已经到底了。
            let fresh = list
                .iter()
                .filter(|v| {
                    v.get("aweme_id")
                        .and_then(|x| x.as_str())
                        .map(|id| !seen_ids.contains(id))
                        .unwrap_or(true)
                })
                .count();
            for v in &list {
                if let Some(id) = v.get("aweme_id").and_then(|x| x.as_str()) {
                    seen_ids.insert(id.to_string());
                }
            }
            if !empty_page {
                count += fresh as u32;
                on_page(page, &list);
            }
            // 原始 JSON 到此为止：回调已经把需要的数据取走，立即释放
            drop(list);

            if empty_page {
                empty_pages += 1;
                // 连续五页都是空页才认作到底：单页空在分片边界上很常见，
                // 一页空就停会截断结果。
                if empty_pages >= 5 {
                    break;
                }
            } else {
                empty_pages = 0;
                if fresh == 0 {
                    repeat_pages += 1;
                    // 连续三页全无新内容：游标在推进但数据没变，已经到底
                    if repeat_pages >= 3 {
                        break;
                    }
                } else {
                    repeat_pages = 0;
                }
            }

            let prev_cursor = max_cursor;
            max_cursor = resp.data.max_cursor.unwrap_or(0);
            let has_more = resp.data.has_more;
            page += 1;

            let elapsed = started.elapsed().as_secs_f64();
            let delay = (elapsed * 0.5).clamp(0.1, 1.0);
            tokio::time::sleep(Duration::from_secs_f64(delay)).await;

            // has_more 只作为提示，不当作终止条件。
            //
            // 服务端在数据量大时会中途抛出 has_more=0（限流、分片、A/B 桶都可能），
            // 一旦据此 break，几千个作品只落几百个就停。
            let _ = has_more;
            // 游标不前进时必须停：否则同一页会被无限重复抓取，
            // 表现是「越抓越慢、内存一直涨，作品数却不再增加」。
            if max_cursor == prev_cursor {
                break;
            }
            // 游标绕圈：服务端在列表尾部会让游标归零并从第一页重放，
            // 只靠「不前进」拦不住，必须认出「这个游标我已经来过」。
            if !visited_cursors.insert(max_cursor) {
                break;
            }
            // 安全阀：游标一路前进但服务端始终非空时，避免无限翻页。
            if page > MAX_PAGES {
                warning = Some(format!(
                    "已翻到 {MAX_PAGES} 页并主动停止，结果可能不完整"
                ));
                break;
            }
        }
        Ok(FetchOutcome {
            count,
            warning,
            stopped,
        })
    }

    /// 收藏夹：POST，分页参数在 body。
    async fn fetch_collect<F, S>(
        &self,
        sec_user_id: &str,
        mut should_stop: S,
        mut on_page: F,
    ) -> AppResult<FetchOutcome>
    where
        F: FnMut(u32, &[Value]),
        S: FnMut() -> bool,
    {
        let base = self.endpoint("/aweme/v1/web/aweme/listcollection/");
        let mut count = 0u32;
        let mut page = 1u32;
        let mut cursor = 0i64;
        let mut page_retry = 0u32;
        let mut warning: Option<String> = None;
        let mut stopped = false;
        // 同主页路径：用「本页是否有新 id」判断到底
        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut repeat_pages = 0u32;
        // 连续空页计数：单页空不终止
        let mut empty_pages = 0u32;
        // 记录走过的游标值：列表尾部服务端会让游标归零重放
        let mut visited_cursors: std::collections::HashSet<i64> =
            std::collections::HashSet::from([0i64]);

        loop {
            if should_stop() {
                stopped = true;
                break;
            }
            let qs = urlencode(&RequestProfile::collect_query(sec_user_id));
            let body = RequestProfile::collect_body(PAGE_COUNT, cursor);
            // 签名时传入 body：收藏接口会校验 body 的 a_bogus
            let (signed, _) = self.sim.sign(&qs, &body);
            let url = format!("{base}?{signed}");

            // 同主页路径：单页失败保住已抓到的页
            let page_result = async {
                let resp = self
                    .client
                    .post(&url)
                    .header("Cookie", &self.cookie)
                    .header("Referer", "https://www.douyin.com/")
                    .header("x-tt-argus", "1")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(body)
                    .send()
                    .await
                    .map_err(AppError::from)?;

                if !resp.status().is_success() {
                    return Err(AppError::msg(format!(
                        "收藏列表请求失败 HTTP {}",
                        resp.status()
                    )));
                }
                let text = resp.text().await.map_err(AppError::from)?;
                let root: Value = serde_json::from_str(&text)
                    .map_err(|e| AppError::msg(format!("收藏列表解析失败: {e}")))?;
                Ok::<_, AppError>(root)
            }
            .await;

            let root = match page_result {
                Ok(r) => r,
                Err(e) => {
                    if count == 0 {
                        return Err(e);
                    }
                    if page_retry < MAX_PAGE_RETRIES {
                        page_retry += 1;
                        tokio::time::sleep(retry_backoff(page_retry)).await;
                        continue;
                    }
                    warning = Some(format!("第 {page} 页中断（{e}），已保留前 {}", page - 1));
                    break;
                }
            };

            let code = root.get("status_code").and_then(|v| v.as_i64()).unwrap_or(0);

            // 先取数据再判错误码：收藏接口在「没有更多」时会返回 status_code=5，
            // 但响应中仍可能携带本页数据，因此先取数据再判断错误码。
            let list: Vec<Value> = root
                .get("aweme_list")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if code != 0 && list.is_empty() {
                // 业务码 5 = 该账号没有收藏（或未公开），当作空结果而非错误
                if code == 5 {
                    break;
                }
                // 3002279 在收藏接口上恒表示「请求的不是本人账号」，
                // 是永久性错误，必须在退避重试之前判断：
                // 它的数值同时又满足 is_rate_limited，若放到后面，
                // 用户会白白等满 5+10+20+40+60 秒才看到真正的提示。
                if code == 3002279 {
                    let e = AppError::msg(
                        "收藏作品只能获取当前登录账号的收藏。请留空链接直接获取，或先登录该账号",
                    );
                    if count == 0 {
                        return Err(e);
                    }
                    warning = Some(format!("第 {page} 页中断（{e}），已保留前 {}", page - 1));
                    break;
                }
                // 风控：退避后重试，避免用户看到吓人的长码。
                // 大列表上单次重试不够，会把后面几千条全丢掉。
                if is_rate_limited(code) && page_retry < MAX_PAGE_RETRIES {
                    page_retry += 1;
                    tokio::time::sleep(retry_backoff(page_retry)).await;
                    continue;
                }
                let detail = describe_code(code);
                if count == 0 {
                    return Err(AppError::msg(detail));
                }
                warning = Some(format!("第 {page} 页中断（{detail}），已保留前 {}", page - 1));
                break;
            }
            page_retry = 0;
            if code != 0 {
                log_status_once(code);
            }

            let list_was_empty = list.is_empty();
            if list_was_empty {
                // 空页不等于到底（同主页路径）：单页空要能继续翻。
                empty_pages += 1;
                if empty_pages >= 5 {
                    break;
                }
            } else {
                empty_pages = 0;
            }
            // 与主页路径同规则：游标在前进但内容全是见过的 id，说明已到底
            let fresh = list
                .iter()
                .filter(|v| {
                    v.get("aweme_id")
                        .and_then(|x| x.as_str())
                        .map(|id| !seen_ids.contains(id))
                        .unwrap_or(true)
                })
                .count();
            for v in &list {
                if let Some(id) = v.get("aweme_id").and_then(|x| x.as_str()) {
                    seen_ids.insert(id.to_string());
                }
            }
            if !list.is_empty() {
                count += fresh as u32;
                on_page(page, &list);
            }
            drop(list);

            if fresh == 0 && !list_was_empty {
                repeat_pages += 1;
                if repeat_pages >= 3 {
                    break;
                }
            } else if !list_was_empty {
                repeat_pages = 0;
            }

            let has_more = lenient_field_bool(root.get("has_more"));
            let next = lenient_field_i64(root.get("cursor"))
                .or_else(|| lenient_field_i64(root.get("max_cursor")))
                .unwrap_or(cursor);

            // 同主页路径：has_more 不作为终止条件，只看游标是否前进。
            // 收藏接口在数据量大时会提前给出 has_more=0，据此 break 会少抓一大半。
            let _ = has_more;
            // cursor 不前进时必须停：否则同一页会被无限重复抓取
            if next == cursor {
                break;
            }
            // 游标绕圈：列表尾部归零重放时必须认出已来过的游标
            if !visited_cursors.insert(next) {
                break;
            }
            cursor = next;
            page += 1;
            if page > MAX_PAGES {
                warning = Some(format!(
                    "已翻到 {MAX_PAGES} 页并主动停止，结果可能不完整"
                ));
                break;
            }

            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        Ok(FetchOutcome {
            count,
            warning,
            stopped,
        })
    }
}

/// 非致命业务码只提示一次，避免刷屏。
fn log_status_once(code: i64) {
    use std::sync::atomic::{AtomicI64, Ordering};
    static LAST: AtomicI64 = AtomicI64::new(i64::MIN);
    if LAST.swap(code, Ordering::Relaxed) != code {
        eprintln!("[douyin] 收藏列表返回非零业务码 {code}，已按部分数据继续");
    }
}

/// 把 JSON 值宽松地读成 bool。
///
/// 接口在不同版本里用 `1/0`、`true/false`、`"1"/"true"` 表达同一个开关。
/// 收藏路径直接操作 `serde_json::Value`，这里补上与结构体侧一致的解释规则。
fn lenient_field_bool(v: Option<&Value>) -> bool {
    match v {
        None => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(Value::String(s)) => {
            let t = s.trim();
            t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes")
        }
        _ => false,
    }
}

/// 把 JSON 值宽松地读成 i64（游标可能是数字或字符串）。
fn lenient_field_i64(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                return None;
            }
            t.parse::<i64>().ok().or_else(|| t.parse::<f64>().ok().map(|f| f as i64))
        }
        _ => None,
    }
}

/// 把抖音的业务码翻译成用户能看懂的话。
///
/// 3002279 这类长码是风控限流，特点是**几分钟后自己会好**——
/// 直接甩码给用户会让人以为是程序坏了，反复点击还会延长冷却时间。
pub fn describe_code(code: i64) -> String {
    match code {
        5 => "该账号没有收藏内容，或收藏未公开".to_string(),
        8 => "该列表需要登录，请先在侧栏登录".to_string(),
        2150 => "请求过于频繁，已被抖音限流，请稍后再试".to_string(),
        3002279 => "请求过于频繁（抖音风控），等 1~2 分钟再试，期间请勿连续点击".to_string(),
        c if c >= 3_000_000 => {
            format!("请求被抖音风控拦截（status_code={c}），等 1~2 分钟再试，期间请勿连续点击")
        }
        c => format!("接口返回 status_code={c}，稍后重试；若持续失败请重新登录"),
    }
}

/// 是否属于「等一下就好」的风控类错误。
pub fn is_rate_limited(code: i64) -> bool {
    matches!(code, 2150 | 3002279) || code >= 3_000_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_from_full_url() {
        assert_eq!(
            extract_sec_user_id("https://www.douyin.com/user/MS4wLjABAAAAxyz?from=xx"),
            Some("MS4wLjABAAAAxyz".into())
        );
        assert_eq!(
            extract_sec_user_id("https://x.com/?sec_user_id=MS4wLjABABCDEF"),
            Some("MS4wLjABABCDEF".into())
        );
        assert_eq!(extract_sec_user_id("https://v.douyin.com/iAbCdEf/"), None);
    }

    #[test]
    fn extract_aweme_id_forms() {
        let id = "7300000000000000001";
        assert_eq!(
            extract_aweme_id(&format!("https://www.douyin.com/video/{id}")),
            Some(id.into())
        );
        assert_eq!(
            extract_aweme_id(&format!("https://www.douyin.com/note/{id}")),
            Some(id.into())
        );
        assert_eq!(
            extract_aweme_id(&format!("https://www.douyin.com/user/MS4w?modal_id={id}")),
            Some(id.into())
        );
        assert_eq!(
            extract_aweme_id(&format!("https://x/?aweme_id={id}")),
            Some(id.into())
        );
        assert_eq!(extract_aweme_id("https://v.douyin.com/iAbCdEf/"), None);
    }

    #[test]
    fn urlencode_keeps_key_value_pairs() {
        let p = vec![
            ("a".to_string(), "1".to_string()),
            ("sec_user_id".to_string(), "MS4wLjABAAA_-x".to_string()),
            ("empty".to_string(), "".to_string()),
        ];
        assert_eq!(urlencode(&p), "a=1&sec_user_id=MS4wLjABAAA_-x&empty=");
    }

    #[test]
    fn urlencode_escapes_like_quote_plus() {
        let p = vec![
            ("k".to_string(), "中文 空格".to_string()),
            ("q".to_string(), "a&b=c".to_string()),
        ];
        assert_eq!(
            urlencode(&p),
            "k=%E4%B8%AD%E6%96%87+%E7%A9%BA%E6%A0%BC&q=a%26b%3Dc"
        );
    }

    #[test]
    fn post_params_has_expected_shape() {
        let p = RequestProfile::post_params("SID", 123, 50, true);
        let map: std::collections::HashMap<_, _> = p.into_iter().collect();
        assert_eq!(map["aid"], "6383");
        assert_eq!(map["max_cursor"], "123");
        assert_eq!(map["need_time_list"], "1");
        assert_eq!(map["version_code"], "290100");
    }

    #[test]
    fn detail_params_carry_aweme_id() {
        let p = RequestProfile::detail_params("7300000000000000001");
        let map: std::collections::HashMap<_, _> = p.into_iter().collect();
        assert_eq!(map["aweme_id"], "7300000000000000001");
        assert_eq!(map["aid"], "6383");
    }

    #[test]
    fn rate_limit_codes_are_described_readably() {
        // 风控码不能让用户看到裸数字
        let d = describe_code(3002279);
        assert!(d.contains("风控"), "{d}");
        assert!(d.contains("再试"), "{d}");
        assert!(!d.contains("3002279") || d.contains("风控"));

        assert!(describe_code(5).contains("收藏"));
        assert!(describe_code(8).contains("登录"));

        assert!(is_rate_limited(3002279));
        assert!(is_rate_limited(4000001));
        assert!(is_rate_limited(2150));
        assert!(!is_rate_limited(5));
        assert!(!is_rate_limited(0));
    }

    #[test]
    fn collect_uses_post_body_for_paging() {
        // 收藏接口的 count/cursor 在 body 里，query 只带通用指纹
        let q = RequestProfile::collect_query("SEC");
        let qm: std::collections::HashMap<_, _> = q.into_iter().collect();
        assert_eq!(qm["sec_user_id"], "SEC");
        assert!(!qm.contains_key("count"), "count 不应出现在 query");

        // count 超过 20 会被服务端拒绝，必须夹紧
        assert_eq!(RequestProfile::collect_body(50, 0), "count=20&cursor=0");
        assert_eq!(RequestProfile::collect_body(20, 12345), "count=20&cursor=12345");
        assert_eq!(RequestProfile::collect_body(5, 0), "count=5&cursor=0");
    }
}
