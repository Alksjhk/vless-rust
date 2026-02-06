//! 标准 Base64 编码实现（RFC 4648）
//! 仅实现编码功能，用于 WebSocket 握手

/// 标准 Base64 编码
pub fn encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut output = Vec::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let mut buffer = [0u8; 3];
        buffer[..chunk.len()].copy_from_slice(chunk);

        // 将3字节转换为4个6位索引
        let b0 = buffer[0] as u32;
        let b1 = buffer[1] as u32;
        let b2 = buffer[2] as u32;

        let combined = (b0 << 16) | (b1 << 8) | b2;

        output.push(TABLE[((combined >> 18) & 0x3F) as usize]);
        output.push(TABLE[((combined >> 12) & 0x3F) as usize]);
        output.push(TABLE[((combined >> 6) & 0x3F) as usize]);
        output.push(TABLE[(combined & 0x3F) as usize]);
    }

    // 处理填充
    let rem = input.len() % 3;
    if rem == 1 {
        let len = output.len();
        output.truncate(len - 2);
        output.push(b'=');
        output.push(b'=');
    } else if rem == 2 {
        let len = output.len();
        output.truncate(len - 1);
        output.push(b'=');
    }

    unsafe { String::from_utf8_unchecked(output) }
}
