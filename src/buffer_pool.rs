//! 缓冲区池实现
//!
//! 使用对象池模式复用缓冲区，减少内存分配

use object_pool::Pool;
use std::sync::Arc;

/// 缓冲区池
#[derive(Clone)]
pub struct BufferPool {
    pool: Arc<Pool<Vec<u8>>>,
    buffer_size: usize,
}

impl BufferPool {
    // 缓冲区大小限制：1KB ~ 16MB
    const MIN_BUFFER_SIZE: usize = 1024;
    const MAX_BUFFER_SIZE: usize = 16 * 1024 * 1024;

    // 池大小限制：1 ~ 256
    const MIN_POOL_SIZE: usize = 1;
    const MAX_POOL_SIZE: usize = 256;

    /// 创建新的缓冲区池
    ///
    /// # 参数
    /// - `buffer_size`: 每个缓冲区的大小（字节），将被限制在 1KB ~ 16MB
    /// - `pool_size`: 池中缓冲区的数量，将被限制在 1 ~ 256
    pub fn new(buffer_size: usize, pool_size: usize) -> Self {
        // 验证并限制参数范围，防止无效配置
        let buffer_size = buffer_size.clamp(Self::MIN_BUFFER_SIZE, Self::MAX_BUFFER_SIZE);
        let pool_size = pool_size.clamp(Self::MIN_POOL_SIZE, Self::MAX_POOL_SIZE);

        let pool = Pool::new(pool_size, || vec![0u8; buffer_size]);
        Self {
            pool: Arc::new(pool),
            buffer_size,
        }
    }

    /// 租借缓冲区（使用 guard 自动归还）
    ///
    /// 返回的 Reusable guard 会在 drop 时自动归还缓冲区到池中
    pub fn acquire(&self) -> Reusable<'_, Vec<u8>> {
        self.pool.pull(|| vec![0u8; self.buffer_size])
    }

    /// 获取缓冲区大小（返回实际缓冲区大小）
    ///
    /// 此方法保留用于 API 完整性和调试目的
    #[allow(dead_code)]
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

// Re-export for convenience
pub use object_pool::Reusable;
