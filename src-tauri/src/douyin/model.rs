//! 抖音接口的响应模型。全部 `Option` + `#[serde(default)]`：
//! 单一字段改名只会让对应作品被跳过，不会中断整页解析。

use serde::Deserialize;


/// 把 JSON `null` 反序列化为 `T::default()`，用于兼容接口把列表字段写成 null 的情况。
fn null_to_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Default,
{
    let v = Option::<T>::deserialize(d)?;
    Ok(v.unwrap_or_default())
}

/// 宽松布尔：接口在不同版本里用 `1/0`、`true/false`、`"1"/"true"` 表示同一个开关。
///
/// 只按数字解析时，字符串形态会让整页结构体反序列化失败——一页都拿不到，
/// 而调用方只看到「解析失败」。这里把三种形态归一，未知值按 false（保守：当作没有更多）。
fn lenient_bool<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Bool(bool),
        Int(i64),
        Float(f64),
        Str(String),
    }

    let v = Option::<Raw>::deserialize(d)?;
    Ok(match v {
        None => false,
        Some(Raw::Bool(b)) => b,
        Some(Raw::Int(i)) => i != 0,
        Some(Raw::Float(f)) => f != 0.0,
        Some(Raw::Str(s)) => {
            let t = s.trim();
            t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes")
        }
    })
}

/// 宽松整数：游标在不同版本里可能是数字，也可能是字符串。
///
/// 直接写 `Option<i64>` 时，字符串游标会让整个响应解析失败，
/// 表现为「抓几十页后突然报解析失败」。
fn lenient_i64<'de, D>(d: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Int(i64),
        Float(f64),
        Str(String),
    }

    let v = Option::<Raw>::deserialize(d)?;
    Ok(match v {
        None => None,
        Some(Raw::Int(i)) => Some(i),
        Some(Raw::Float(f)) => Some(f as i64),
        Some(Raw::Str(s)) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<i64>().ok().or_else(|| t.parse::<f64>().ok().map(|f| f as i64))
            }
        }
    })
}

#[derive(Debug, Deserialize, Default)]
pub struct ApiResponse<T> {
    #[serde(default)]
    pub status_code: i64,
    #[serde(default)]
    pub status_msg: Option<String>,
    #[serde(flatten)]
    pub data: T,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProfileData {
    #[serde(default)]
    pub user: Option<ProfileUser>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ProfileUser {
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub unique_id: Option<String>,
    /// 主页的唯一标识，拼 https://www.douyin.com/user/{sec_uid} 即可访问
    #[serde(default)]
    pub sec_uid: Option<String>,
    #[serde(default)]
    pub aweme_count: Option<i64>,
    /// 头像三档（小/中/大），取最后一档最清晰。
    #[serde(default)]
    pub avatar_larger: Option<UrlList>,
    #[serde(default)]
    pub avatar_medium: Option<UrlList>,
    #[serde(default)]
    pub avatar_thumb: Option<UrlList>,
}

impl ProfileUser {
    pub fn best_avatar(&self) -> String {
        for src in [
            self.avatar_larger.as_ref(),
            self.avatar_medium.as_ref(),
            self.avatar_thumb.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(u) = src.url_list.first().filter(|u| !u.is_empty()) {
                return u.clone();
            }
        }
        String::new()
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PostData {
    #[serde(default)]
    pub aweme_list: Option<Vec<serde_json::Value>>,
    #[serde(default, deserialize_with = "lenient_i64")]
    pub max_cursor: Option<i64>,
    #[serde(default, deserialize_with = "lenient_bool")]
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct UrlList {
    #[serde(default, deserialize_with = "null_to_default")]
    pub url_list: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct PlayAddr {
    #[serde(default, deserialize_with = "null_to_default")]
    pub url_list: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct BitRate {
    #[serde(default)]
    pub bit_rate: i64,
    #[serde(default)]
    pub play_addr: PlayAddr,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct VideoInfo {
    /// 图集作品的顶层 `video` 常把 `bit_rate` 写成 `null`。
    /// `#[serde(default)]` 只处理字段缺失，遇到显式 null 会报
    /// `invalid type: null, expected a sequence` 并让整条作品解析失败，
    /// 因此用 `null_to_default` 把 null 归一化为空列表。
    #[serde(default, deserialize_with = "null_to_default")]
    pub bit_rate: Vec<BitRate>,
    #[serde(default)]
    pub duration: i64,
    #[serde(default)]
    pub play_addr: Option<PlayAddr>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ImageInfo {
    #[serde(default, deserialize_with = "null_to_default")]
    pub url_list: Vec<String>,
    #[serde(default)]
    pub video: Option<VideoInfo>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct MixInfo {
    #[serde(default)]
    pub mix_name: Option<String>,
    #[serde(default)]
    pub mix_name_str: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Statistics {
    #[serde(default)]
    pub digg_count: i64,
    #[serde(default)]
    pub comment_count: i64,
    #[serde(default)]
    pub collect_count: i64,
    #[serde(default)]
    pub share_count: i64,
    #[serde(default)]
    pub recommend_count: i64,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Author {
    #[serde(default)]
    pub unique_id: Option<String>,
    #[serde(default)]
    pub sec_uid: Option<String>,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub avatar_larger: Option<UrlList>,
    #[serde(default)]
    pub avatar_medium: Option<UrlList>,
    #[serde(default)]
    pub avatar_thumb: Option<UrlList>,
}

impl Author {
    pub fn best_avatar(&self) -> String {
        for src in [
            self.avatar_larger.as_ref(),
            self.avatar_medium.as_ref(),
            self.avatar_thumb.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(u) = src.url_list.first().filter(|u| !u.is_empty()) {
                return u.clone();
            }
        }
        String::new()
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Aweme {
    #[serde(default)]
    pub aweme_id: Option<String>,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub create_time: Option<i64>,
    #[serde(default)]
    pub video: Option<VideoInfo>,
    #[serde(default)]
    pub images: Option<Vec<ImageInfo>>,
    #[serde(default)]
    pub mix_info: Option<MixInfo>,
    #[serde(default)]
    pub mix_name: Option<String>,
    #[serde(default)]
    pub mix_name_str: Option<String>,
    #[serde(default)]
    pub statistics: Option<Statistics>,
    #[serde(default)]
    pub author: Option<Author>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_prefers_larger() {
        let u: ProfileUser = serde_json::from_value(serde_json::json!({
            "nickname": "n",
            "avatar_thumb": {"url_list": ["https://t/1.jpg"]},
            "avatar_larger": {"url_list": ["https://l/1.jpg"]}
        }))
        .unwrap();
        assert_eq!(u.best_avatar(), "https://l/1.jpg");
    }

    #[test]
    fn avatar_falls_back_and_tolerates_missing() {
        let u: ProfileUser = serde_json::from_value(serde_json::json!({
            "avatar_thumb": {"url_list": ["https://t/1.jpg"]}
        }))
        .unwrap();
        assert_eq!(u.best_avatar(), "https://t/1.jpg");

        let empty = ProfileUser::default();
        assert_eq!(empty.best_avatar(), "");
    }
}
