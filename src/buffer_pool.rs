//! 缓冲区池实现
//!
//! 使用互斥锁和向量池实现，提供更可控的内存分配策略

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};

/// 可复用的缓冲区 Guard
///
/// 当 Guard 被 drop 时，缓冲区会自动归还到池中
pub struct Reusable<'a> {
    pool: &'a BufferPool,
    data: Option<Vec<u8>>,
}

impl<'a> Reusable<'a> {
    /// 消费 Guard 并返回内部数据（不归还到池中）
    #[allow(dead_code)]
    pub fn detach(mut self) -> Vec<u8> {
        self.data.take().expect("Buffer already consumed")
    }
}

impl<'a> Drop for Reusable<'a> {
    fn drop(&mut self) {
        if let Some(data) = self.data.take() {
            self.pool.return_buffer(data);
        }
    }
}

impl<'a> Deref for Reusable<'a> {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        self.data.as_ref().expect("Buffer already consumed")
    }
}

impl<'a> DerefMut for Reusable<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().expect("Buffer already consumed")
    }
}

/// 缓冲区池
#[derive(Clone)]
pub struct BufferPool {
    inner: Arc<BufferPoolInner>,
}

struct BufferPoolInner {
    buffers: Mutex<VecDeque<Vec<u8>>>,
    buffer_size: usize,
    max_size: usize,
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
    /// - `pool_size`: 池中缓冲区的最大数量，将被限制在 1 ~ 256
    pub fn new(buffer_size: usize, pool_size: usize) -> Self {
        // 验证并限制参数范围，防止无效配置
        let buffer_size = buffer_size.clamp(Self::MIN_BUFFER_SIZE, Self::MAX_BUFFER_SIZE);
        let pool_size = pool_size.clamp(Self::MIN_POOL_SIZE, Self::MAX_POOL_SIZE);

        let inner = BufferPoolInner {
            buffers: Mutex::new(VecDeque::with_capacity(pool_size)),
            buffer_size,
            max_size: pool_size,
        };

        Self {
            inner: Arc::new(inner),
        }
    }

    /// 租借缓冲区（使用 guard 自动归还）
    ///
    /// 返回的 Reusable guard 会在 drop 时自动归还缓冲区到池中
    ///
    /// # 性能说明
    /// 此实现使用 Mutex 保护内部队列，避免过度分配：
    /// - 首次访问会创建新缓冲区
    /// - 后续访问会优先复用池中缓冲区
    /// - 池满时，归还的缓冲区会被丢弃（释放内存）
    pub fn acquire(&self) -> Reusable<'_> {
        let mut buffers = self.inner.buffers.lock().unwrap();

        let data = if let Some(mut buf) = buffers.pop_front() {
            // 复用池中的缓冲区，清空内容
            buf.clear();
            buf.resize(self.inner.buffer_size, 0);
            buf
        } else {
            // 池为空，创建新缓冲区
            vec![0u8; self.inner.buffer_size]
        };

        Reusable {
            pool: self,
            data: Some(data),
        }
    }

    /// 归还缓冲区到池中
    fn return_buffer(&self, mut buffer: Vec<u8>) {
        let mut buffers = self.inner.buffers.lock().unwrap();

        // 如果池未满，归还缓冲区；否则丢弃
        if buffers.len() < self.inner.max_size {
            buffer.clear();
            buffers.push_back(buffer);
        }
        // 池满时，直接 drop buffer，释放内存
    }

    /// 获取缓冲区大小（返回实际缓冲区大小）
    ///
    /// 此方法保留用于 API 完整性和调试目的
    #[allow(dead_code)]
    pub fn buffer_size(&self) -> usize {
        self.inner.buffer_size
    }

    /// 获取当前池中的缓冲区数量（用于调试和监控）
    #[allow(dead_code)]
    pub fn pool_size(&self) -> usize {
        self.inner.buffers.lock().unwrap().len()
    }
}
