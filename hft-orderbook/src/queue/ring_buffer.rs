//! Ring buffer implementation inspired by LMAX Disruptor pattern.
//! Triển khai này tối ưu hiệu năng nhưng vẫn đảm bảo an toàn bộ nhớ theo tiêu chuẩn Rust.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::fmt;
use parking_lot::Mutex;
use std::mem::MaybeUninit;

/// Cache line size in bytes (thường là 64 bytes trên CPU hiện đại)
const CACHE_LINE_SIZE: usize = 64;

/// Padding để tránh false sharing giữa các biến quan trọng
#[repr(align(64))]
struct CachePadding {
    _data: [u8; CACHE_LINE_SIZE],
}

impl CachePadding {
    const fn new() -> Self {
        Self { _data: [0; CACHE_LINE_SIZE] }
    }
}

impl fmt::Debug for CachePadding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CachePadding").field(&"...").finish()
    }
}

/// Sequence counter với cache line padding để tránh false sharing
#[repr(align(64))]
#[derive(Debug)]
struct PaddedSequence {
    _pad1: CachePadding,
    sequence: AtomicUsize,
    _pad2: CachePadding,
}

impl PaddedSequence {
    const fn new(value: usize) -> Self {
        Self {
            _pad1: CachePadding::new(),
            sequence: AtomicUsize::new(value),
            _pad2: CachePadding::new(),
        }
    }

    #[inline]
    fn get(&self) -> usize {
        self.sequence.load(Ordering::Acquire)
    }

    #[inline]
    fn set(&self, value: usize) {
        self.sequence.store(value, Ordering::Release);
    }

    #[inline]
    fn increment_and_get(&self, increment: usize) -> usize {
        self.sequence.fetch_add(increment, Ordering::AcqRel) + increment
    }
}

/// Cell cho một phần tử trong ring buffer
#[derive(Debug)]
struct Cell<T> {
    /// Dữ liệu được bảo vệ bởi mutex nhẹ (parking_lot)
    data: Mutex<MaybeUninit<T>>,
}

impl<T> Cell<T> {
    fn new() -> Self {
        Self {
            data: Mutex::new(MaybeUninit::uninit()),
        }
    }

    /// Lưu một giá trị vào cell
    #[inline]
    fn store(&self, value: T) {
        let mut guard = self.data.lock();
        *guard = MaybeUninit::new(value);
    }

    /// Lấy một bản sao của giá trị từ cell
    #[inline]
    fn load_clone(&self) -> T where T: Clone {
        let guard = self.data.lock();
        unsafe { guard.assume_init_ref().clone() }
    }

    /// Thực thi một hàm với tham chiếu không thay đổi đến giá trị
    #[inline]
    fn with_ref<F, R>(&self, f: F) -> R 
    where 
        F: FnOnce(&T) -> R 
    {
        let guard = self.data.lock();
        let value = unsafe { guard.assume_init_ref() };
        f(value)
    }

    /// Thực thi một hàm với tham chiếu có thể thay đổi đến giá trị
    #[inline]
    fn with_mut<F, R>(&self, f: F) -> R 
    where 
        F: FnOnce(&mut T) -> R 
    {
        let mut guard = self.data.lock();
        let value = unsafe { guard.assume_init_mut() };
        f(value)
    }

    /// Lấy giá trị từ cell và để lại trạng thái chưa khởi tạo
    #[inline]
    fn take(&self) -> T {
        let mut guard = self.data.lock();
        let mut uninit = MaybeUninit::uninit();
        std::mem::swap(&mut *guard, &mut uninit);
        unsafe { uninit.assume_init() }
    }

    /// Cập nhật giá trị trong cell
    #[inline]
    fn update<F>(&self, f: F) 
    where 
        F: FnOnce(&mut T)
    {
        self.with_mut(f);
    }
}

/// Ring buffer cho phép luồng sản xuất (producer) và tiêu thụ (consumer) hoạt động đồng thời
#[derive(Debug)]
pub struct OrderRingBuffer<T> {
    /// Buffer lưu các phần tử
    buffer: Box<[Cell<T>]>,
    /// Mặt nạ để thực hiện phép toán modulo nhanh (capacity phải là lũy thừa của 2)
    mask: usize,
    /// Cursor cho vị trí ghi tiếp theo
    next_write: PaddedSequence,
    /// Cursor cho vị trí đã commit
    committed: PaddedSequence,
    /// Cursor cho vị trí đọc tiếp theo
    next_read: PaddedSequence,
}

impl<T> OrderRingBuffer<T> {
    /// Tạo một ring buffer mới với capacity cho trước
    pub fn new(capacity: usize) -> Self {
        // Đảm bảo capacity là lũy thừa của 2 để sử dụng mask
        let capacity = capacity.next_power_of_two();
        
        // Khởi tạo buffer với các cell chưa được khởi tạo
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(Cell::new());
        }
        
        Self {
            buffer: buffer.into_boxed_slice(),
            mask: capacity - 1,
            next_write: PaddedSequence::new(0),
            committed: PaddedSequence::new(0),
            next_read: PaddedSequence::new(0),
        }
    }

    /// Lấy dung lượng của buffer
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Yêu cầu slot tiếp theo để ghi
    #[inline]
    pub fn claim(&self) -> (usize, usize) {
        let sequence = self.next_write.increment_and_get(1);
        let index = (sequence - 1) & self.mask;
        (sequence - 1, index)
    }

    /// Yêu cầu nhiều slot để ghi (batch)
    #[inline]
    pub fn claim_batch(&self, n: usize) -> (usize, usize) {
        let sequence = self.next_write.increment_and_get(n);
        let start_index = (sequence - n) & self.mask;
        (sequence - n, start_index)
    }

    /// Publish một phần tử vào buffer
    #[inline]
    pub fn publish(&self, sequence: usize, value: T) {
        let index = sequence & self.mask;
        
        // Ghi giá trị vào buffer
        self.buffer[index].store(value);
        
        // Chờ cho đến khi các sequence trước đó đã được commit
        while self.committed.get() != sequence {
            std::hint::spin_loop();
        }
        
        // Commit sequence này
        self.committed.set(sequence + 1);
    }

    /// Publish trực tiếp không qua claim (cho trường hợp single producer)
    #[inline]
    pub fn publish_direct(&self, value: T) -> usize {
        let sequence = self.next_write.increment_and_get(1) - 1;
        let index = sequence & self.mask;
        
        // Ghi giá trị vào buffer
        self.buffer[index].store(value);
        
        // Commit sequence này
        self.committed.set(sequence + 1);
        
        sequence
    }

    /// Publish nhiều phần tử cùng lúc (batch)
    #[inline]
    pub fn publish_batch(&self, start_sequence: usize, values: &[T]) 
    where 
        T: Clone
    {
        for (i, value) in values.iter().enumerate() {
            let sequence = start_sequence + i;
            let index = sequence & self.mask;
            self.buffer[index].store(value.clone());
        }
        
        // Chờ cho đến khi các sequence trước đó đã được commit
        while self.committed.get() != start_sequence {
            std::hint::spin_loop();
        }
        
        // Commit tất cả sequence này
        self.committed.set(start_sequence + values.len());
    }

    /// Thử lấy phần tử tiếp theo để đọc
    #[inline]
    pub fn try_next(&self) -> Option<(usize, T)> {
        let current = self.next_read.get();
        
        // Kiểm tra xem có giá trị committed để đọc hay không
        if current < self.committed.get() {
            let index = current & self.mask;
            
            // Đọc giá trị từ buffer
            let value = self.buffer[index].take();
            
            // Cập nhật vị trí đọc
            self.next_read.set(current + 1);
            
            Some((current, value))
        } else {
            None
        }
    }

    /// Thử lấy nhiều phần tử để đọc (batch)
    #[inline]
    pub fn try_next_batch(&self, max_batch_size: usize) -> Vec<(usize, T)> {
        let current = self.next_read.get();
        let available = self.committed.get().saturating_sub(current);
        
        if available == 0 {
            return Vec::new();
        }
        
        let batch_size = available.min(max_batch_size);
        let mut result = Vec::with_capacity(batch_size);
        
        for i in 0..batch_size {
            let sequence = current + i;
            let index = sequence & self.mask;
            
            // Đọc giá trị từ buffer
            let value = self.buffer[index].take();
            result.push((sequence, value));
        }
        
        // Cập nhật vị trí đọc
        self.next_read.set(current + batch_size);
        
        result
    }

    /// Xóa một phần tử khỏi buffer
    #[inline]
    pub fn remove(&self, sequence: usize) -> Option<T> {
        let index = sequence & self.mask;
        
        // Kiểm tra xem sequence có nằm trong phạm vi hiện tại không
        if sequence < self.next_read.get() || sequence >= self.committed.get() {
            return None;
        }
        
        // Lấy giá trị từ buffer
        Some(self.buffer[index].take())
    }

    /// Lấy một bản sao của phần tử từ buffer
    #[inline]
    pub fn get(&self, sequence: usize) -> Option<T> 
    where 
        T: Clone
    {
        let index = sequence & self.mask;
        
        // Kiểm tra xem sequence có nằm trong phạm vi hiện tại không
        if sequence < self.next_read.get() || sequence >= self.committed.get() {
            return None;
        }
        
        // Sao chép giá trị từ buffer
        Some(self.buffer[index].load_clone())
    }

    /// Cập nhật một phần tử trong buffer
    #[inline]
    pub fn update(&self, sequence: usize, updater: impl FnOnce(&mut T)) -> bool {
        let index = sequence & self.mask;
        
        // Kiểm tra xem sequence có nằm trong phạm vi hiện tại không
        if sequence < self.next_read.get() || sequence >= self.committed.get() {
            return false;
        }
        
        // Cập nhật giá trị trong buffer
        self.buffer[index].update(updater);
        
        true
    }

    /// Lấy iterator qua các phần tử có sẵn trong buffer
    pub fn iter(&self) -> OrderRingBufferIterator<'_, T> 
    where 
        T: Clone
    {
        OrderRingBufferIterator {
            buffer: self,
            current: self.next_read.get(),
            end: self.committed.get(),
        }
    }

    /// Kiểm tra xem buffer có trống không
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.next_read.get() == self.committed.get()
    }

    /// Kiểm tra xem buffer có đầy không
    #[inline]
    pub fn is_full(&self) -> bool {
        self.next_write.get() - self.next_read.get() >= self.buffer.len()
    }

    /// Lấy số lượng phần tử hiện có trong buffer
    #[inline]
    pub fn len(&self) -> usize {
        self.committed.get() - self.next_read.get()
    }

    /// Xóa tất cả phần tử trong buffer
    pub fn clear(&self) 
    where 
        T: Clone
    {
        while let Some(_) = self.try_next() {}
    }
}

/// Iterator qua các phần tử trong ring buffer
pub struct OrderRingBufferIterator<'a, T> {
    /// Tham chiếu đến buffer
    buffer: &'a OrderRingBuffer<T>,
    /// Vị trí hiện tại
    current: usize,
    /// Vị trí kết thúc
    end: usize,
}

impl<'a, T: Clone> Iterator for OrderRingBufferIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.end {
            let index = self.current & self.buffer.mask;
            
            // Đọc giá trị từ buffer
            let value = self.buffer.buffer[index].load_clone();
            
            self.current += 1;
            Some(value)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.end - self.current;
        (remaining, Some(remaining))
    }
}

impl<'a, T> ExactSizeIterator for OrderRingBufferIterator<'a, T> 
where 
    T: Clone
{
    fn len(&self) -> usize {
        self.end - self.current
    }
}

impl<'a, T> fmt::Debug for OrderRingBufferIterator<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderRingBufferIterator")
            .field("current", &self.current)
            .field("end", &self.end)
            .finish()
    }
}

// Các tính năng bổ sung cho trường hợp sử dụng Single Producer, Single Consumer (SPSC)
impl<T> OrderRingBuffer<T> {
    /// Phương thức tối ưu hóa cho single producer
    #[inline]
    pub fn push(&self, value: T) -> Result<(), T> {
        if self.is_full() {
            return Err(value);
        }
        
        self.publish_direct(value);
        Ok(())
    }
    
    /// Phương thức tối ưu hóa cho single consumer
    #[inline]
    pub fn pop(&self) -> Option<T> {
        self.try_next().map(|(_, value)| value)
    }
    
    /// Phương thức nhìn trước mà không loại bỏ phần tử
    #[inline]
    pub fn peek(&self) -> Option<T>
    where
        T: Clone
    {
        let current = self.next_read.get();
        if current < self.committed.get() {
            let index = current & self.mask;
            Some(self.buffer[index].load_clone())
        } else {
            None
        }
    }
    
    /// Thử thêm nhiều phần tử cùng lúc
    pub fn push_batch(&self, values: &[T]) -> Result<(), ()>
    where
        T: Clone
    {
        if values.is_empty() {
            return Ok(());
        }
        
        if self.next_write.get() - self.next_read.get() + values.len() > self.buffer.len() {
            return Err(());
        }
        
        let (start_sequence, _) = self.claim_batch(values.len());
        self.publish_batch(start_sequence, values);
        
        Ok(())
    }
    
    /// Thêm một phần tử vào buffer, chờ đợi nếu buffer đầy
    pub fn push_blocking(&self, value: T, spin_limit: usize) -> Result<(), T> {
        let mut spins = 0;
        while self.is_full() {
            spins += 1;
            if spins > spin_limit {
                return Err(value);
            }
            std::hint::spin_loop();
        }
        
        self.publish_direct(value);
        Ok(())
    }
    
    /// Lấy một phần tử từ buffer, chờ đợi nếu buffer trống
    pub fn pop_blocking(&self, spin_limit: usize) -> Option<T> {
        let mut spins = 0;
        while self.is_empty() {
            spins += 1;
            if spins > spin_limit {
                return None;
            }
            std::hint::spin_loop();
        }
        
        self.pop()
    }
}