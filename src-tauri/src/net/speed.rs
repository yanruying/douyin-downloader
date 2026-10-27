//! 全局网卡接收速率采样（sysinfo），500ms 一次。
//!
//! 采样的是整机所有网卡 `total_received` 的增量之和——即用户任务管理器里
//! 「以太网 / WLAN 接收」的量级，而非本进程的下载量。

use std::sync::Arc;
use std::time::Duration;

use sysinfo::Networks;
use tokio::sync::Mutex;

use crate::models::SpeedEvent;

pub const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);
const NETWORKS_REFRESH: Duration = Duration::from_millis(200);

/// 返回当前接收速率（B/s）。第一次调用返回 0（无基线）。
pub struct SpeedSampler {
    networks: Networks,
    last_total: u64,
    last_at: Option<std::time::Instant>,
}

impl SpeedSampler {
    pub fn new() -> Self {
        let mut n = Networks::new_with_refreshed_list();
        n.refresh(false);
        let total: u64 = n.list().values().map(|d| d.total_received()).sum();
        Self {
            networks: n,
            last_total: total,
            last_at: None,
        }
    }

    pub fn sample(&mut self) -> u64 {
        self.networks.refresh(false);
        let total: u64 = self.networks.list().values().map(|d| d.total_received()).sum();
        let now = std::time::Instant::now();

        let rate = match self.last_at {
            Some(prev) => {
                let dt = now.duration_since(prev).as_secs_f64().max(0.001);
                let delta = total.saturating_sub(self.last_total);
                (delta as f64 / dt) as u64
            }
            None => 0,
        };

        self.last_total = total;
        self.last_at = Some(now);
        rate
    }
}

impl Default for SpeedSampler {
    fn default() -> Self {
        Self::new()
    }
}

/// 常驻采样循环，向 `emit` 推送。取消靠 `AtomicBool`。
pub async fn run_sampler<F>(sampler: Arc<Mutex<SpeedSampler>>, stop: Arc<std::sync::atomic::AtomicBool>, mut emit: F)
where
    F: FnMut(SpeedEvent),
{
    // 先跑一次建立基线，避免首个采样值为天量
    {
        let mut s = sampler.lock().await;
        let _ = s.sample();
    }
    loop {
        tokio::time::sleep(SAMPLE_INTERVAL.max(NETWORKS_REFRESH)).await;
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        let rate = {
            let mut s = sampler.lock().await;
            s.sample()
        };
        emit(SpeedEvent {
            bytes_per_sec: rate,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampler_returns_plausible_values() {
        let mut s = SpeedSampler::new();
        let first = s.sample();
        assert_eq!(first, 0, "首个采样无基线应为 0");
        std::thread::sleep(Duration::from_millis(120));
        let second = s.sample();
        // 只断言量级合理（< 10 Gbps），不依赖机器空闲状态
        assert!(second < 10_000_000_000, "rate={second}");
    }
}
