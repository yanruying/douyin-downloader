pub mod abogus;

/// 签名提供者抽象：抖音换签名时只需新增一个实现，调用方无感。
pub trait SignatureProvider: Send + Sync {
    fn sign(&self, params: &str, body: &str) -> (String, String);
}

/// 进程内单例包装：`ABogus` 有跨调用累积的置换状态，必须串行复用同一份。
pub struct AbogusProvider {
    inner: std::sync::Mutex<abogus::ABogus>,
}

impl AbogusProvider {
    pub fn new(user_agent: Option<String>) -> Self {
        Self {
            inner: std::sync::Mutex::new(abogus::ABogus::new(user_agent, None)),
        }
    }
}

impl SignatureProvider for AbogusProvider {
    fn sign(&self, params: &str, body: &str) -> (String, String) {
        let mut guard = self.inner.lock().expect("abogus mutex poisoned");
        guard.generate(params, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_is_thread_safe_and_reusable() {
        use std::sync::Arc;
        let p: Arc<dyn SignatureProvider> = Arc::new(AbogusProvider::new(None));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let p = p.clone();
            handles.push(std::thread::spawn(move || p.sign("a=1&b=2", "").1));
        }
        let sigs: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(sigs.len(), 8);
        assert!(sigs.iter().all(|s| !s.is_empty()));
        // 同一实例连续签名长度应稳定（随机填充长度固定）
        let lens: std::collections::HashSet<usize> = sigs.iter().map(|s| s.len()).collect();
        assert_eq!(lens.len(), 1, "{lens:?}");
    }
}
