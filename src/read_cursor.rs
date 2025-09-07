pub(crate) struct ReadCursor<'a>(pub &'a [u8]);

impl<'a> ReadCursor<'a> {
    pub fn next_bytes<const N: usize>(&mut self) -> Option<&'a [u8; N]> {
        let (bytes, next) = self.0.split_first_chunk()?;
        self.0 = next;
        Some(bytes)
    }
    pub fn next_n<T: bytemuck::AnyBitPattern + bytemuck::NoUninit>(&mut self, n: usize) -> &'a [T] {
        let (this, next) = bytemuck::cast_slice(self.0).split_at(n);
        self.0 = bytemuck::cast_slice(next);
        this
    }
    pub fn next_u8(&mut self) -> Option<u8> {
        let (&byte, next) = self.0.split_first()?;
        self.0 = next;
        Some(byte)
    }
    pub fn next_u32_le(&mut self) -> Option<u32> {
        self.next_bytes().copied().map(u32::from_le_bytes)
    }
    pub fn skip(&mut self, amount: usize) {
        self.0 = &self.0[amount..];
    }
}
