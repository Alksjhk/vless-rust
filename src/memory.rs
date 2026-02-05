//! 内存池模块
//!
//! 提供高性能的内存池实现，减少频繁的内存分配和释放
//! 使用对象池模式管理缓冲区，提升并发性能

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};

/// 内存池统计信息
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_allocated: usize,
    pub total_returned: usize,
    pub current_pool_size: usize,
    pub peak_pool_size: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

/// 缓冲区包装器
pub struct PooledBuffer {
    buffer: Vec<u8>,
    pool: Arc<BufferPool>,
    returned: bool,
}

impl PooledBuffer {
    fn new(buffer: Vec<u8>, pool: Arc<BufferPool>) -> Self {
        Self {
            buffer,
            pool,
            returned: false,
        }
    }

    /// 获取缓冲区的可变引用
    pub fn as_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    /// 获取缓冲区的不可变引用
    #[allow(dead_code)]
    pub fn as_ref(&self) -> &Vec<u8> {
        &self.buffer
    }

    /// 获取缓冲区长度
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// 检查缓冲区是否为空
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// 清空缓冲区内容但保留容量
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// 调整缓冲区大小
    #[allow(dead_code)]
    pub fn resize(&mut self, new_len: usize, value: u8) {
        self.buffer.resize(new_len, value);
    }

    /// 手动归还缓冲区到池中
    #[allow(dead_code)]
    pub fn return_to_pool(mut self) {
        if !self.returned {
            self.returned = true;
            self.pool.return_buffer(std::mem::take(&mut self.buffer));
        }
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if !self.returned {
            self.returned = true;
            self.pool.return_buffer(std::mem::take(&mut self.buffer));
        }
    }
}

impl std::ops::Deref for PooledBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl std::ops::DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

/// 高性能缓冲区池
pub struct BufferPool {
    /// 缓冲区队列
    buffers: Mutex<VecDeque<Vec<u8>>>,
    /// 缓冲区大小
    buffer_size: usize,
    /// 最大池大小
    max_pool_size: usize,
    /// 统计信息
    stats: Mutex<PoolStats>,
    /// 原子计数器
    allocated_count: AtomicUsize,
    returned_count: AtomicUsize,
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

impl BufferPool {
    /// 创建新的缓冲区池
    ///
    /// # 参数
    /// * `buffer_size` - 每个缓冲区的大小（字节）
    /// * `initial_size` - 初始预分配的缓冲区数量
    /// * `max_pool_size` - 最大池大小，防止内存泄漏
    pub fn new(buffer_size: usize, initial_size: usize, max_pool_size: usize) -> Arc<Self> {
        let pool = Arc::new(Self {
            buffers: Mutex::new(VecDeque::with_capacity(initial_size)),
            buffer_size,
            max_pool_size,
            stats: Mutex::new(PoolStats {
                total_allocated: 0,
                total_returned: 0,
                current_pool_size: 0,
                peak_pool_size: 0,
                cache_hits: 0,
                cache_misses: 0,
            }),
            allocated_count: AtomicUsize::new(0),
            returned_count: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        });

        // 预分配缓冲区
        {
            let mut buffers = pool.buffers.lock().unwrap();
            for _ in 0..initial_size {
                buffers.push_back(vec![0u8; buffer_size]);
            }
            let mut stats = pool.stats.lock().unwrap();
            stats.current_pool_size = initial_size;
            stats.peak_pool_size = initial_size;
        }

        debug!(
            "Created buffer pool: size={}, initial={}, max={}",
            buffer_size, initial_size, max_pool_size
        );

        pool
    }

    /// 从池中获取缓冲区
    pub fn get_buffer(self: &Arc<Self>) -> PooledBuffer {
        let buffer = {
            let mut buffers = self.buffers.lock().unwrap();
            if let Some(mut buf) = buffers.pop_front() {
                // 从池中获取现有缓冲区
                buf.clear();
                buf.resize(self.buffer_size, 0);
                self.cache_hits.fetch_add(1, Ordering::Relaxed);

                // 更新统计
                let mut stats = self.stats.lock().unwrap();
                stats.current_pool_size = buffers.len();
                stats.cache_hits += 1;

                buf
            } else {
                // 池为空，创建新缓冲区
                self.cache_misses.fetch_add(1, Ordering::Relaxed);

                // 更新统计
                let mut stats = self.stats.lock().unwrap();
                stats.cache_misses += 1;

                vec![0u8; self.buffer_size]
            }
        };

        self.allocated_count.fetch_add(1, Ordering::Relaxed);

        // 更新总分配统计
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_allocated += 1;
        }

        PooledBuffer::new(buffer, Arc::clone(self))
    }

    /// 归还缓冲区到池中
    fn return_buffer(&self, buffer: Vec<u8>) {
        self.returned_count.fetch_add(1, Ordering::Relaxed);

        let mut buffers = self.buffers.lock().unwrap();

        // 检查池大小限制
        if buffers.len() < self.max_pool_size {
            buffers.push_back(buffer);

            // 更新统计
            let mut stats = self.stats.lock().unwrap();
            stats.current_pool_size = buffers.len();
            if stats.current_pool_size > stats.peak_pool_size {
                stats.peak_pool_size = stats.current_pool_size;
            }
            stats.total_returned += 1;
        } else {
            // 池已满，丢弃缓冲区
            warn!("Buffer pool is full, discarding buffer");
            let mut stats = self.stats.lock().unwrap();
            stats.total_returned += 1;
        }
    }

    /// 获取池统计信息
    #[allow(dead_code)]
    pub fn get_stats(&self) -> PoolStats {
        let mut stats = self.stats.lock().unwrap();
        stats.total_allocated = self.allocated_count.load(Ordering::Relaxed);
        stats.total_returned = self.returned_count.load(Ordering::Relaxed);
        stats.cache_hits = self.cache_hits.load(Ordering::Relaxed);
        stats.cache_misses = self.cache_misses.load(Ordering::Relaxed);
        stats.clone()
    }

    /// 清空池中的所有缓冲区
    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut buffers = self.buffers.lock().unwrap();
        buffers.clear();

        let mut stats = self.stats.lock().unwrap();
        stats.current_pool_size = 0;

        debug!("Buffer pool cleared");
    }

    /// 获取当前池大小
    #[allow(dead_code)]
    pub fn current_size(&self) -> usize {
        let buffers = self.buffers.lock().unwrap();
        buffers.len()
    }

    /// 获取缓冲区大小
    #[allow(dead_code)]
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

/// 全局缓冲区池管理器
pub struct GlobalBufferPools {
    /// 小缓冲区池 (4KB)
    small_pool: Arc<BufferPool>,
    /// 中等缓冲区池 (64KB)
    medium_pool: Arc<BufferPool>,
    /// 大缓冲区池 (128KB)
    large_pool: Arc<BufferPool>,
}

impl GlobalBufferPools {
    /// 创建全局缓冲区池管理器
    pub fn new() -> Self {
        Self {
            small_pool: BufferPool::new(4 * 1024, 50, 200), // 4KB, 初始50个, 最大200个
            medium_pool: BufferPool::new(64 * 1024, 20, 100), // 64KB, 初始20个, 最大100个
            large_pool: BufferPool::new(128 * 1024, 10, 50), // 128KB, 初始10个, 最大50个
        }
    }

    /// 根据大小获取合适的缓冲区
    pub fn get_buffer(&self, size: usize) -> PooledBuffer {
        if size <= 4 * 1024 {
            self.small_pool.get_buffer()
        } else if size <= 64 * 1024 {
            self.medium_pool.get_buffer()
        } else {
            self.large_pool.get_buffer()
        }
    }

    /// 获取小缓冲区 (4KB)
    #[allow(dead_code)]
    pub fn get_small_buffer(&self) -> PooledBuffer {
        self.small_pool.get_buffer()
    }

    /// 获取中等缓冲区 (64KB)
    #[allow(dead_code)]
    pub fn get_medium_buffer(&self) -> PooledBuffer {
        self.medium_pool.get_buffer()
    }

    /// 获取大缓冲区 (128KB)
    #[allow(dead_code)]
    pub fn get_large_buffer(&self) -> PooledBuffer {
        self.large_pool.get_buffer()
    }

    /// 获取所有池的统计信息
    #[allow(dead_code)]
    pub fn get_all_stats(&self) -> (PoolStats, PoolStats, PoolStats) {
        (
            self.small_pool.get_stats(),
            self.medium_pool.get_stats(),
            self.large_pool.get_stats(),
        )
    }

    /// 清空所有池
    #[allow(dead_code)]
    pub fn clear_all(&self) {
        self.small_pool.clear();
        self.medium_pool.clear();
        self.large_pool.clear();
    }
}

impl Default for GlobalBufferPools {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_buffer_pool_basic() {
        let pool = BufferPool::new(1024, 2, 10);

        // 获取缓冲区
        let buf1 = pool.get_buffer();
        assert_eq!(buf1.len(), 1024);

        let buf2 = pool.get_buffer();
        assert_eq!(buf2.len(), 1024);

        // 检查统计
        let stats = pool.get_stats();
        assert_eq!(stats.total_allocated, 2);
        // cache_hits can be 0 or more, depending on pool state
    }

    #[test]
    fn test_buffer_pool_reuse() {
        let pool = BufferPool::new(1024, 1, 10);

        // 获取并归还缓冲区
        {
            let _buf = pool.get_buffer();
        } // 自动归还

        // 再次获取应该重用
        let _buf2 = pool.get_buffer();

        let stats = pool.get_stats();
        assert_eq!(stats.total_allocated, 2);
        assert_eq!(stats.total_returned, 1);
        assert!(stats.cache_hits > 0);
    }

    #[test]
    fn test_global_pools() {
        let pools = GlobalBufferPools::new();

        let small = pools.get_small_buffer();
        let medium = pools.get_medium_buffer();
        let large = pools.get_large_buffer();

        assert_eq!(small.len(), 4 * 1024);
        assert_eq!(medium.len(), 64 * 1024);
        assert_eq!(large.len(), 128 * 1024);
    }

    #[test]
    fn test_concurrent_access() {
        let pool = BufferPool::new(1024, 5, 20);
        let pool_clone = Arc::clone(&pool);

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let pool = Arc::clone(&pool_clone);
                thread::spawn(move || {
                    for _ in 0..10 {
                        let _buf = pool.get_buffer();
                        thread::sleep(Duration::from_millis(1));
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = pool.get_stats();
        assert_eq!(stats.total_allocated, 100);
    }
}
