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
    /// 创建新的缓冲区池
    ///
    /// # 参数
    /// - `buffer_size`: 每个缓冲区的大小（字节）
    /// - `pool_size`: 池中缓冲区的数量
    pub fn new(buffer_size: usize, pool_size: usize) -> Self {
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

    /// 获取缓冲区大小（保留用于API完整性）
    #[allow(dead_code)]
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

// Re-export for convenience
pub use object_pool::Reusable;
