use serde::Serialize;

/// 统一错误类型：前端只收 `{ code, message }`，避免把 reqwest/serde 细节漏出去。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Msg(String),
    #[error("网络请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Tauri 内部错误: {0}")]
    Tauri(#[from] tauri::Error),
}

impl AppError {
    pub fn msg(s: impl Into<String>) -> Self {
        AppError::Msg(s.into())
    }

    fn code(&self) -> &'static str {
        match self {
            AppError::Msg(_) => "APP",
            AppError::Http(_) => "HTTP",
            AppError::Io(_) => "IO",
            AppError::Json(_) => "JSON",
            AppError::Tauri(_) => "TAURI",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
