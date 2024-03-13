// Copyright 2022 Ryan Seipp
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Raw Request iterator
use core::fmt::Display;
use core::slice;

/// TODO
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Skip
    Skip,
    /// Take
    Take,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Skip is greater than current position")
    }
}

impl std::error::Error for Error {}

/// TODO
#[derive(Debug, Clone)]
pub struct RawRequest<'a> {
    inner: &'a [u8],
    pos: usize,
}

/// TODO
impl<'a> RawRequest<'a> {
    /// TODO
    pub fn new(slice: &'a [u8]) -> Self {
        RawRequest {
            inner: slice,
            pos: 0,
        }
    }

    /// TODO
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// TODO
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// TODO
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos == self.inner.len()
    }

    /// TODO
    #[inline]
    pub fn current(&self) -> Option<u8> {
        return self
            .inner
            .get(if self.pos == 0 { 0 } else { self.pos - 1 })
            .copied();
    }

    /// to_vec
    pub fn to_vec(&self) -> Vec<u8> {
        self.inner.to_vec()
    }

    /// TODO
    #[inline]
    pub fn peek(&self) -> Option<u8> {
        return self.inner.get(self.pos).copied();
    }

    /// TODO
    #[inline]
    pub fn advance(&mut self, n: usize) {
        self.pos = self.inner.len().min(self.pos + n);
    }

    /// TODO
    #[inline]
    pub fn slice(&mut self) -> &'a [u8] {
        return self.slice_skip(0).expect("slice_skip shall not fail");
    }

    /// TODO
    #[inline]
    pub fn slice_skip(&mut self, skip: usize) -> Result<&'a [u8], Error> {
        if skip > self.pos {
            return Err(Error::Skip);
        }

        let head_pos = self.pos - skip;
        let ptr = self.inner.as_ptr();

        // SAFETY: head_pos is guaranteed to be in (0,self.pos], so we only create a new slice from
        // within current slice.
        let head = unsafe { slice::from_raw_parts(ptr, head_pos) };

        // SAFETY: self.pos is guaranteed to be `<= self.len()`, so tail is guaranteed to be the
        // remainder of the slice, or have zero length.
        let tail = unsafe { slice::from_raw_parts(ptr.add(self.pos), self.inner.len() - self.pos) };
        self.pos = 0;
        self.inner = tail;

        Ok(head)
    }

    /// TODO
    #[inline]
    pub fn take_until<F>(&mut self, mut predicate: F) -> Option<&'a [u8]>
    where
        F: FnMut(u8) -> bool,
    {
        loop {
            match self.peek() {
                Some(b) if predicate(b) => {
                    let slice = self.slice();
                    return if slice.is_empty() { None } else { Some(slice) };
                }
                Some(_) => {
                    self.next();
                }
                None => {
                    self.slice();
                    // TODO: may be a bug if slice returns non-empty slice
                    return None;
                }
            }
            // if let Some(b) = self.peek() {
            //     if predicate(b) {
            //         let slice = self.slice();
            //         if slice.is_empty() {
            //             return None;
            //         } else {
            //             return Some(slice);
            //         }
            //     }
            //     self.next();
            // } else {
            //     self.slice();
            //     return None;
            // }
        }
    }
}

impl<'a> Iterator for RawRequest<'a> {
    type Item = &'a u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.inner.get(self.pos);
        if result.is_some() {
            self.pos += 1;
        }

        result
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for RawRequest<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }
}

impl<'a> AsRef<[u8]> for RawRequest<'a> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.inner[self.pos..]
    }
}

#[cfg(test)]
mod test {

    use super::{Error, RawRequest};

    #[test]
    fn raw_request_constructs_with_len_and_pos() {
        let req = RawRequest::new(b"GET / HTTP/1.1");
        assert_eq!(0, req.pos());
        assert_eq!(14, req.len());
    }

    #[test]
    fn raw_request_next_iterates() {
        let mut req = RawRequest::new(b"GET / HTTP/1.1");
        assert_eq!(Some(&b'G'), req.next());
        assert_eq!(Some(&b'E'), req.next());
        assert_eq!(Some(&b'T'), req.next());
        assert_eq!(3, req.pos());
    }

    #[test]
    fn raw_request_slice_consumes_iterated_elements() {
        let mut req = RawRequest::new(b"GET / HTTP/1.1");
        req.next();
        req.next();
        req.next();
        assert_eq!(b"GET", req.slice());
        assert_eq!(0, req.pos());
        assert_eq!(11, req.len());
    }

    #[test]
    fn raw_request_slice_skip_consumes_iterated_elements_and_skips() {
        let mut req = RawRequest::new(b"GET / HTTP/1.1");
        req.next();
        req.next();
        req.next();
        req.next();
        assert_eq!(Ok(b"GET" as &[u8]), req.slice_skip(1));
        assert_eq!(0, req.pos());
        assert_eq!(10, req.len());
    }

    #[test]
    fn raw_request_slice_skip_returns_empty_slice_when_skip_equals_pos() {
        let mut req = RawRequest::new(b"GET / HTTP/1.1");
        req.next();
        req.next();
        req.next();
        req.next();
        assert_eq!(4, req.pos());
        assert_eq!(Ok(b"" as &[u8]), req.slice_skip(4));
        assert_eq!(0, req.pos());
        assert_eq!(10, req.len());
    }

    #[test]
    fn raw_requset_slice_skip_returns_err_when_skip_greater_than_pos() {
        let mut req = RawRequest::new(b"GET / HTTP/1.1");
        assert_eq!(Err(Error::Skip), req.slice_skip(1));
    }

    #[test]
    fn raw_request_slice_leaves_empty_slice_when_all_elements_consumed() {
        let mut req = RawRequest::new(b"GET");
        req.next();
        req.next();
        req.next();
        assert_eq!(b"GET", req.slice());
        assert_eq!(0, req.pos());
        assert_eq!(0, req.len());
        assert_eq!(None, req.next());
    }
}
