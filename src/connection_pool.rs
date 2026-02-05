//! 连接池管理模块
//!
//! 提供高性能的连接池实现，支持连接复用和负载均衡
//! 减少连接建立开销，提升并发性能

use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

/// 连接池统计信息
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_created: usize,
    pub total_reused: usize,
    pub total_closed: usize,
    pub current_active: usize,
    pub current_idle: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

/// 池化连接包装器
pub struct PooledConnection {
    stream: Option<TcpStream>,
    pool: Arc<ConnectionPool>,
    target_addr: SocketAddr,
    created_at: Instant,
    last_used: Instant,
    returned: bool,
}

impl PooledConnection {
    fn new(stream: TcpStream, pool: Arc<ConnectionPool>, target_addr: SocketAddr) -> Self {
        let now = Instant::now();
        Self {
            stream: Some(stream),
            pool,
            target_addr,
            created_at: now,
            last_used: now,
            returned: false,
        }
    }

    /// 获取底层TCP流
    pub fn into_stream(mut self) -> Option<TcpStream> {
        self.returned = true;
        self.stream.take()
    }

    /// 获取连接年龄
    #[allow(dead_code)]
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// 获取空闲时间
    #[allow(dead_code)]
    pub fn idle_time(&self) -> Duration {
        self.last_used.elapsed()
    }

    /// 手动归还连接到池中
    #[allow(dead_code)]
    pub async fn return_to_pool(mut self) {
        if !self.returned && self.stream.is_some() {
            self.returned = true;
            if let Some(stream) = self.stream.take() {
                self.pool.return_connection(stream, self.target_addr).await;
            }
        }
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if !self.returned && self.stream.is_some() {
            let stream = self.stream.take().unwrap();
            let pool = Arc::clone(&self.pool);
            let target_addr = self.target_addr;

            // 异步归还连接
            tokio::spawn(async move {
                pool.return_connection(stream, target_addr).await;
            });
        }
    }
}

/// 连接池条目
struct PoolEntry {
    stream: TcpStream,
    created_at: Instant,
    last_used: Instant,
}

impl PoolEntry {
    fn new(stream: TcpStream) -> Self {
        let now = Instant::now();
        Self {
            stream,
            created_at: now,
            last_used: now,
        }
    }

    fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    fn idle_time(&self) -> Duration {
        self.last_used.elapsed()
    }

    fn touch(&mut self) {
        self.last_used = Instant::now();
    }
}

/// 连接池配置
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 每个目标地址的最大连接数
    pub max_connections_per_host: usize,
    /// 连接的最大空闲时间
    pub max_idle_time: Duration,
    /// 连接的最大生存时间
    pub max_lifetime: Duration,
    /// 连接建立超时时间
    pub connect_timeout: Duration,
    /// 连接健康检查间隔
    pub health_check_interval: Duration,
    /// 是否启用连接预热
    pub enable_warmup: bool,
    /// 预热连接数量
    pub warmup_connections: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_host: 10,
            max_idle_time: Duration::from_secs(300), // 5分钟
            max_lifetime: Duration::from_secs(3600), // 1小时
            connect_timeout: Duration::from_secs(10),
            health_check_interval: Duration::from_secs(60),
            enable_warmup: false,
            warmup_connections: 2,
        }
    }
}

/// 高性能连接池
pub struct ConnectionPool {
    /// 按目标地址分组的连接池
    pools: RwLock<HashMap<SocketAddr, Mutex<VecDeque<PoolEntry>>>>,
    /// 配置
    config: PoolConfig,
    /// 统计信息
    stats: Mutex<PoolStats>,
    /// 原子计数器
    total_created: AtomicUsize,
    total_reused: AtomicUsize,
    total_closed: AtomicUsize,
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

impl ConnectionPool {
    /// 创建新的连接池
    pub fn new(config: PoolConfig) -> Arc<Self> {
        let pool = Arc::new(Self {
            pools: RwLock::new(HashMap::new()),
            config,
            stats: Mutex::new(PoolStats {
                total_created: 0,
                total_reused: 0,
                total_closed: 0,
                current_active: 0,
                current_idle: 0,
                cache_hits: 0,
                cache_misses: 0,
            }),
            total_created: AtomicUsize::new(0),
            total_reused: AtomicUsize::new(0),
            total_closed: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        });

        // 启动后台清理任务
        let pool_clone = Arc::clone(&pool);
        tokio::spawn(async move {
            pool_clone.cleanup_task().await;
        });

        pool
    }

    /// 获取连接
    pub async fn get_connection(
        self: &Arc<Self>,
        target_addr: SocketAddr,
    ) -> Result<PooledConnection> {
        // 首先尝试从池中获取现有连接
        if let Some(stream) = self.try_get_pooled_connection(target_addr).await {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            self.total_reused.fetch_add(1, Ordering::Relaxed);
            debug!("Reused pooled connection to {}", target_addr);
            return Ok(PooledConnection::new(stream, Arc::clone(self), target_addr));
        }

        // 池中没有可用连接，创建新连接
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        let stream = self.create_new_connection(target_addr).await?;
        self.total_created.fetch_add(1, Ordering::Relaxed);

        debug!("Created new connection to {}", target_addr);
        Ok(PooledConnection::new(stream, Arc::clone(self), target_addr))
    }

    /// 尝试从池中获取连接
    async fn try_get_pooled_connection(&self, target_addr: SocketAddr) -> Option<TcpStream> {
        let pools = self.pools.read().await;
        if let Some(pool_mutex) = pools.get(&target_addr) {
            let mut pool = pool_mutex.lock().await;

            // 查找健康的连接
            while let Some(mut entry) = pool.pop_front() {
                // 检查连接是否过期
                if entry.age() > self.config.max_lifetime
                    || entry.idle_time() > self.config.max_idle_time
                {
                    // 连接过期，丢弃
                    self.total_closed.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                // 简单的连接健康检查
                if self.is_connection_healthy(&entry.stream).await {
                    entry.touch();
                    return Some(entry.stream);
                } else {
                    // 连接不健康，丢弃
                    self.total_closed.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        None
    }

    /// 创建新连接
    async fn create_new_connection(&self, target_addr: SocketAddr) -> Result<TcpStream> {
        let stream =
            tokio::time::timeout(self.config.connect_timeout, TcpStream::connect(target_addr))
                .await??;

        // 配置TCP参数
        stream.set_nodelay(true)?;

        Ok(stream)
    }

    /// 简单的连接健康检查
    async fn is_connection_healthy(&self, _stream: &TcpStream) -> bool {
        // 简化的健康检查 - 在实际应用中可以发送ping或检查socket状态
        // 这里假设连接是健康的，实际实现可以检查socket的可读/可写状态
        true
    }

    /// 归还连接到池中
    async fn return_connection(&self, stream: TcpStream, target_addr: SocketAddr) {
        // 检查连接是否仍然健康
        if !self.is_connection_healthy(&stream).await {
            self.total_closed.fetch_add(1, Ordering::Relaxed);
            return;
        }

        let pools = self.pools.read().await;
        if let Some(pool_mutex) = pools.get(&target_addr) {
            let mut pool = pool_mutex.lock().await;

            // 检查池大小限制
            if pool.len() < self.config.max_connections_per_host {
                pool.push_back(PoolEntry::new(stream));
                debug!("Returned connection to pool for {}", target_addr);
            } else {
                // 池已满，关闭连接
                self.total_closed.fetch_add(1, Ordering::Relaxed);
                debug!("Pool full, closing connection to {}", target_addr);
            }
        } else {
            // 为新的目标地址创建池
            drop(pools);
            let mut pools = self.pools.write().await;
            let pool_mutex = pools
                .entry(target_addr)
                .or_insert_with(|| Mutex::new(VecDeque::new()));
            let mut pool = pool_mutex.lock().await;
            pool.push_back(PoolEntry::new(stream));
            debug!(
                "Created new pool and returned connection for {}",
                target_addr
            );
        }
    }

    /// 预热连接池
    #[allow(dead_code)]
    pub async fn warmup(&self, target_addrs: Vec<SocketAddr>) -> Result<()> {
        if !self.config.enable_warmup {
            return Ok(());
        }

        info!(
            "Warming up connection pools for {} targets",
            target_addrs.len()
        );

        for target_addr in target_addrs {
            for _ in 0..self.config.warmup_connections {
                match self.create_new_connection(target_addr).await {
                    Ok(stream) => {
                        self.return_connection(stream, target_addr).await;
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create warmup connection to {}: {}",
                            target_addr, e
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> PoolStats {
        let pools = self.pools.read().await;
        let current_idle = pools
            .values()
            .map(|pool_mutex| {
                // 这里使用try_lock避免阻塞，如果锁被占用则返回0
                pool_mutex.try_lock().map(|pool| pool.len()).unwrap_or(0)
            })
            .sum();

        let mut stats = self.stats.lock().await;
        stats.total_created = self.total_created.load(Ordering::Relaxed);
        stats.total_reused = self.total_reused.load(Ordering::Relaxed);
        stats.total_closed = self.total_closed.load(Ordering::Relaxed);
        stats.current_idle = current_idle;
        stats.cache_hits = self.cache_hits.load(Ordering::Relaxed);
        stats.cache_misses = self.cache_misses.load(Ordering::Relaxed);

        stats.clone()
    }

    /// 清理过期连接的后台任务
    async fn cleanup_task(&self) {
        let mut interval = tokio::time::interval(self.config.health_check_interval);

        loop {
            interval.tick().await;
            self.cleanup_expired_connections().await;
        }
    }

    /// 清理过期连接
    async fn cleanup_expired_connections(&self) {
        let pools = self.pools.read().await;
        let mut total_cleaned = 0;

        for (target_addr, pool_mutex) in pools.iter() {
            let mut pool = pool_mutex.lock().await;
            let original_len = pool.len();

            // 保留未过期的连接
            pool.retain(|entry| {
                let expired = entry.age() > self.config.max_lifetime
                    || entry.idle_time() > self.config.max_idle_time;
                if expired {
                    self.total_closed.fetch_add(1, Ordering::Relaxed);
                }
                !expired
            });

            let cleaned = original_len - pool.len();
            if cleaned > 0 {
                total_cleaned += cleaned;
                debug!(
                    "Cleaned {} expired connections for {}",
                    cleaned, target_addr
                );
            }
        }

        if total_cleaned > 0 {
            info!("Cleaned {} expired connections total", total_cleaned);
        }
    }

    /// 关闭所有连接池
    #[allow(dead_code)]
    pub async fn shutdown(&self) {
        info!("Shutting down connection pools");
        let mut pools = self.pools.write().await;

        for (target_addr, pool_mutex) in pools.drain() {
            let mut pool = pool_mutex.lock().await;
            let count = pool.len();
            pool.clear();
            self.total_closed.fetch_add(count, Ordering::Relaxed);
            debug!("Closed {} connections for {}", count, target_addr);
        }
    }
}

/// 全局连接池管理器
pub struct GlobalConnectionPools {
    /// 主连接池
    main_pool: Arc<ConnectionPool>,
}

impl GlobalConnectionPools {
    /// 创建全局连接池管理器
    pub fn new() -> Self {
        let config = PoolConfig {
            max_connections_per_host: 20,
            max_idle_time: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(1800),
            connect_timeout: Duration::from_secs(10),
            health_check_interval: Duration::from_secs(30),
            enable_warmup: true,
            warmup_connections: 3,
        };

        Self {
            main_pool: ConnectionPool::new(config),
        }
    }

    /// 获取连接
    pub async fn get_connection(&self, target_addr: SocketAddr) -> Result<PooledConnection> {
        self.main_pool.get_connection(target_addr).await
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> PoolStats {
        self.main_pool.get_stats().await
    }

    /// 预热连接池
    pub async fn warmup(&self, target_addrs: Vec<SocketAddr>) -> Result<()> {
        self.main_pool.warmup(target_addrs).await
    }

    /// 关闭所有连接池
    pub async fn shutdown(&self) {
        self.main_pool.shutdown().await;
    }
}

impl Default for GlobalConnectionPools {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_connection_pool_basic() {
        let config = PoolConfig {
            max_connections_per_host: 5,
            max_idle_time: Duration::from_secs(60),
            max_lifetime: Duration::from_secs(300),
            connect_timeout: Duration::from_secs(5),
            health_check_interval: Duration::from_secs(10),
            enable_warmup: false,
            warmup_connections: 0,
        };

        let pool = ConnectionPool::new(config);
        let _target_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 80);

        // 注意：这个测试需要目标地址可连接才能通过
        // 在实际测试中，可以使用mock或测试服务器
        let stats = pool.get_stats().await;
        assert_eq!(stats.total_created, 0);
        assert_eq!(stats.total_reused, 0);
    }

    #[tokio::test]
    async fn test_global_pools() {
        let pools = GlobalConnectionPools::new();
        let stats = pools.get_stats().await;

        // 初始状态检查
        assert_eq!(stats.total_created, 0);
        assert_eq!(stats.cache_hits, 0);
    }
}
