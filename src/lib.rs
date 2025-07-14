//! A non-owning version of `std::io::Take` that wraps an existing reader by reference,
//! allowing limited reads without transferring ownership.
//!
//! # Example
//!
//! ```
//! use std::io::{Cursor, Read};
//! use reftake::RefTakeExt;
//!
//! let mut cursor = Cursor::new(b"hello world");
//! let mut take = cursor.take_ref(5);
//!
//! let mut buf = String::new();
//! take.read_to_string(&mut buf).unwrap();
//! assert_eq!(buf, "hello");
//!
//! let mut buf2 = String::new();
//! cursor.read_to_string(&mut buf2).unwrap();
//! assert_eq!(buf2, " world");
//! ```
use std::{
    cmp,
    io::{BufRead, Read},
};

/// A non-owning adapter that wraps a mutable reference to a reader,
/// limiting the number of bytes that can be read from it.
///
/// Unlike `std::io::Take`, this version does not take ownership of the reader,
/// allowing continued use of the original reader after wrapping.
///
/// Useful in scenarios where ownership cannot be moved, such as within
/// streaming parsers, frameworks, or when working with borrowed readers.
pub struct RefTake<'a, R> {
    inner: &'a mut R,
    limit: u64,
}

impl<'a, R> RefTake<'a, R> {
    /// Creates a new `RefTake` that reads at most `limit` bytes from the given reader reference.
    ///
    /// # Arguments
    ///
    /// * `inner` - A mutable reference to a type that implements `Read` or `BufRead`.
    /// * `limit` - The maximum number of bytes that can be read from the reader.
    ///
    /// # Returns
    ///
    /// A `RefTake` wrapper that enforces the given byte limit.
    pub fn wrap(inner: &'a mut R, limit: u64) -> Self {
        Self { inner, limit }
    }

    /// Sets a new byte limit for the reader.
    ///
    /// This overrides the current limit, allowing the wrapped reader
    /// to return more data up to the new limit.
    ///
    /// # Arguments
    ///
    /// * `limit` - The new byte limit to enforce.
    pub fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }

    /// Returns the current limit that is allowed to read.
    pub fn current_limit(&self) -> u64 {
        self.limit
    }
}

/// Implements the `Read` trait with a byte limit.
///
/// This ensures no more than the configured number of bytes are read.
/// When the limit is reached, it returns `Ok(0)` (EOF behavior).
///
/// If the inner reader returns more bytes than allowed, it will panic.
impl<T: Read> Read for RefTake<'_, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        // Don't call into inner reader at all at EOF because it may still block
        if self.limit == 0 {
            return Ok(0);
        }

        let max = cmp::min(buf.len() as u64, self.limit) as usize;
        let n = self.inner.read(&mut buf[..max])?;
        assert!(n as u64 <= self.limit, "number of read bytes exceeds limit");
        self.limit -= n as u64;
        Ok(n)
    }
}

/// Implements the `BufRead` trait with a byte limit.
///
/// `fill_buf()` returns a slice of the buffer capped at the remaining limit,
/// and `consume()` ensures the internal limit is decremented correctly.
///
/// Over-consuming more than the limit is clamped and does not cause errors.
impl<T: BufRead> BufRead for RefTake<'_, T> {
    fn fill_buf(&mut self) -> Result<&[u8], std::io::Error> {
        // Don't call into inner reader at all at EOF because it may still block
        if self.limit == 0 {
            return Ok(&[]);
        }

        let buf = self.inner.fill_buf()?;
        let cap = cmp::min(buf.len() as u64, self.limit) as usize;
        Ok(&buf[..cap])
    }

    fn consume(&mut self, amt: usize) {
        // Don't let callers reset the limit by passing an overlarge value
        let amt = cmp::min(amt as u64, self.limit) as usize;
        self.limit -= amt as u64;
        self.inner.consume(amt);
    }
}

/// Extension trait to provide a `take_ref` method on all `Read` types.
pub trait RefTakeExt {
    /// Wraps the reader in a `RefTake`, allowing limited reading via a mutable reference.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of bytes to allow through the wrapper.
    ///
    /// # Example
    ///
    /// ```
    /// use std::io::{Cursor, Read};
    /// use reftake::RefTakeExt;
    ///
    /// let mut cursor = Cursor::new(b"hello world");
    /// let mut take = cursor.take_ref(5);
    ///
    /// let mut buf = String::new();
    /// take.read_to_string(&mut buf).unwrap();
    /// assert_eq!(buf, "hello");
    /// 
    /// ```
    fn take_ref(&mut self, limit: u64) -> RefTake<'_, Self>
    where
        Self: Sized;
}

impl<T: Read> RefTakeExt for T {
    fn take_ref(&mut self, limit: u64) -> RefTake<'_, Self> {
        RefTake::wrap(self, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor, Read};

    #[test]
    fn test_read_respects_limit() {
        let data = b"Hello, world!";
        let mut reader = Cursor::new(data);
        let mut take = reader.take_ref(5);

        let mut buf = [0u8; 10];
        let n = take.read(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf[..n], b"Hello");
        assert_eq!(take.current_limit(), 0);
    }

    #[test]
    fn test_read_in_multiple_calls() {
        let data = b"abcdef";
        let mut reader = Cursor::new(data);
        let mut take = reader.take_ref(6);

        let mut buf1 = [0u8; 2];
        let n1 = take.read(&mut buf1).unwrap();
        assert_eq!(n1, 2);
        assert_eq!(&buf1[..n1], b"ab");
        assert_eq!(take.current_limit(), 4);

        let mut buf2 = [0u8; 3];
        let n2 = take.read(&mut buf2).unwrap();
        assert_eq!(n2, 3);
        assert_eq!(&buf2[..n2], b"cde");
        assert_eq!(take.current_limit(), 1);

        let mut buf3 = [0u8; 4];
        let n3 = take.read(&mut buf3).unwrap();
        assert_eq!(n3, 1);
        assert_eq!(&buf3[..n3], b"f");
        assert_eq!(take.current_limit(), 0);

        let mut buf4 = [0u8; 1];
        let n4 = take.read(&mut buf4).unwrap();
        assert_eq!(n4, 0); // limit reached
        assert_eq!(take.current_limit(), 0);
    }

    #[test]
    fn test_read_zero_limit() {
        let data = b"Hello";
        let mut reader = Cursor::new(data);
        let mut take = reader.take_ref(0);

        let mut buf = [0u8; 5];
        let n = take.read(&mut buf).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn test_set_limit() {
        let data = b"123456789";
        let mut reader = Cursor::new(data);
        let mut take = reader.take_ref(3);

        let mut buf = [0u8; 10];
        let n1 = take.read(&mut buf).unwrap();
        assert_eq!(n1, 3);
        assert_eq!(&buf[..n1], b"123");

        take.set_limit(2);
        let n2 = take.read(&mut buf).unwrap();
        assert_eq!(n2, 2);
        assert_eq!(&buf[..n2], b"45");
    }

    #[test]
    fn test_bufread_fill_buf_respects_limit() {
        let data = b"abcdef";
        let mut reader = BufReader::new(Cursor::new(data));
        let mut take = reader.take_ref(4);

        let buf = take.fill_buf().unwrap();
        assert_eq!(buf, b"abcd");

        take.consume(2);
        let buf2 = take.fill_buf().unwrap();
        assert_eq!(buf2, b"cd");

        take.consume(2);
        let buf3 = take.fill_buf().unwrap();
        assert_eq!(buf3, b"");
    }

    #[test]
    fn test_bufread_consume_does_not_exceed_limit() {
        let data = b"abcde";
        let mut reader = BufReader::new(Cursor::new(data));
        let mut take = reader.take_ref(3);

        let _ = take.fill_buf().unwrap();
        take.consume(10); // should only consume up to 3

        assert_eq!(take.limit, 0);
        let buf = take.fill_buf().unwrap();
        assert_eq!(buf, b"");
    }
}
