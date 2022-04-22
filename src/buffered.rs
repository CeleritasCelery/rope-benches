// This is an experimental approach to jumprope where we buffer all changes


#[derive(Debug, Clone, Copy)]
enum Kind { Ins, Del }

use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut, Range};
use Op::*;
use crate::fast_str_tools::{char_to_byte_idx, count_chars};
use crate::JumpRope;

/// This struct provides an optimized wrapper around JumpRope which buffers adjacent incoming writes
/// before forwarding them to the underlying JumpRope.
///
/// Most of the overhead of writing to a rope comes from finding the edit location in the rope and
/// bookkeeping. Because text editing operations are usually sequential, by aggregating adjacent
/// editing operations together we can amortize the cost of updating the underlying data structure
/// itself. This improves performance by about 10x compared to inserting and deleting individual
/// characters.
///
/// There is nothing jumprope-specific in this library. It could easily be adapted to wrap other
/// rope libraries (like Ropey) too.
///
/// This API is still experimental. This library is only enabled by enabling the "buffered' feature.
pub struct JumpRopeBuf(RefCell<(JumpRope, BufferedOp)>);

#[derive(Debug, Clone)]
struct BufferedOp {
    kind: Kind,
    ins_content: String,
    range: Range<usize>,
}

#[derive(Debug, Clone, Copy)]
enum Op<'a> {
    Ins(usize, &'a str),
    Del(usize, usize), // start, end.
}

impl BufferedOp {
    fn new() -> Self {
        Self {
            kind: Kind::Ins,
            ins_content: "".to_string(),
            range: Range::default(),
        }
    }

    fn is_empty(&self) -> bool {
        // self.len == 0
        self.range.is_empty()
    }

    fn clear(&mut self) {
        // We don't care about the tag.
        self.ins_content.clear();
        self.range = Range::default();
    }

    fn try_append(&mut self, op: Op) -> Result<(), ()> {
        if self.is_empty() {
            // Just set to op.
            match op {
                // I'm setting fields individually here rather than implementing From<Op> or
                // BufferedOp so we can reuse the allocation in self.ins_content.
                Ins(pos, content) => {
                    self.kind = Kind::Ins;
                    self.ins_content.push_str(content);
                    self.range.start = pos;
                    self.range.end = pos + count_chars(content);
                }
                Del(start, end) => {
                    self.kind = Kind::Del;
                    debug_assert!(self.ins_content.is_empty());
                    self.range = start..end;
                }
            }
            Ok(())
        } else {
            match (self.kind, op) {
                (Kind::Ins, Op::Ins(pos, content)) if pos == self.range.end => {
                    // We can merge this.
                    self.ins_content.push_str(content);
                    self.range.end += count_chars(content);
                    Ok(())
                }
                (Kind::Ins, Op::Del(start, end)) if end == self.range.end && start >= self.range.start => {
                    // We can merge if the delete trims the end of the insert. There's more complex
                    // trimming we could do here, but anything too complex and we may as well just
                    // let the rope handle it.
                    if start == self.range.start {
                        // Discard our local insert.
                        self.ins_content.clear();
                        self.range.end = self.range.start;
                        Ok(())
                    } else {
                        // Trim from the end.
                        let char_offset = start - self.range.start;

                        let byte_offset = if self.range.len() == self.ins_content.len() {
                            // If its all ascii, char offset == byte offset.
                            char_offset
                        } else {
                            // TODO: Come up with a better way to calculate this.
                            char_to_byte_idx(self.ins_content.as_str(), char_offset)
                        };

                        self.range.end = start;
                        self.ins_content.truncate(byte_offset);
                        Ok(())
                    }
                }
                (Kind::Del, Op::Del(start, end)) if start <= self.range.start && end >= self.range.start => {
                    // We can merge if our delete is inside the operation.
                    // let self_len = self.range.len();
                    // dbg!(&self.range, (start, end));
                    self.range.end += end - self.range.start;
                    self.range.start = start;
                    Ok(())
                }
                (_, _) => Err(()),
            }
        }
    }
}

impl From<JumpRope> for JumpRopeBuf {
    fn from(rope: JumpRope) -> Self {
        Self::with_rope(rope)
    }
}

impl JumpRopeBuf {
    pub fn with_rope(rope: JumpRope) -> Self {
        Self(RefCell::new((rope, BufferedOp::new())))
    }

    pub fn new() -> Self {
        Self::with_rope(JumpRope::new())
    }

    pub fn new_from_str(s: &str) -> Self {
        Self::with_rope(JumpRope::from(s))
    }

    fn flush_mut(inner: &mut (JumpRope, BufferedOp)) {
        if !inner.1.is_empty() {
            match inner.1.kind {
                Kind::Ins => {
                    inner.0.insert(inner.1.range.start, &inner.1.ins_content);
                },
                Kind::Del => {
                    inner.0.remove(inner.1.range.clone());
                }
            }
            inner.1.clear();
        }
    }

    // fn flush(&self) {
    //     let mut inner = self.0.borrow_mut();
    //     Self::flush_mut(inner.deref_mut());
    // }

    fn internal_push_op(&mut self, op: Op) {
        // let mut inner = self.0.borrow_mut();
        let inner = self.0.get_mut();
        match inner.1.try_append(op) {
            Ok(_) => {}
            Err(_) => {
                // Self::flush_mut(inner.deref_mut());
                Self::flush_mut(inner);
                // inner.0.insert(pos, content);
                inner.1.try_append(op).unwrap();
            }
        }

    }

    pub fn insert(&mut self, pos: usize, content: &str) {
        self.internal_push_op(Op::Ins(pos, content))
    }

    pub fn remove(&mut self, range: Range<usize>) {
        self.internal_push_op(Op::Del(range.start, range.end))
    }

    pub fn len_chars(&self) -> usize {
        let borrow = self.0.borrow();
        match borrow.1.kind {
            Kind::Ins => borrow.0.len_chars() + borrow.1.range.len(),
            Kind::Del => borrow.0.len_chars() - borrow.1.range.len()
        }
    }

    pub fn len_bytes(&self) -> usize {
        let mut borrow = self.0.borrow_mut();
        match borrow.1.kind {
            Kind::Ins => borrow.0.len_bytes() + borrow.1.ins_content.len(),
            Kind::Del => {
                // Unfortunately we have to flush to calculate byte length.
                Self::flush_mut(borrow.deref_mut());
                borrow.0.len_bytes()
            }
        }
    }

    /// Consume the JumpRopeBuf, flushing the contents and returning the contained JumpRope.
    pub fn into_inner(self) -> JumpRope {
        let mut contents = self.0.into_inner();
        Self::flush_mut(&mut contents);
        contents.0
    }

    /// Flush changes into the rope and return a borrowed reference to the Rope itself. This makes
    /// it easy to call any methods on the underlying rope which aren't already exposed through the
    /// buffered API.
    pub fn borrow(&self) -> std::cell::Ref<'_, JumpRope> {
        let mut borrow = self.0.borrow_mut();
        Self::flush_mut(borrow.deref_mut());
        drop(borrow);
        std::cell::Ref::map(self.0.borrow(), |(rope, _)| rope)
    }

    /// Flush changes into the rope and borrow the rope as a &mut JumpRope
    pub fn as_mut(&mut self) -> &'_ mut JumpRope {
        let inner = self.0.get_mut();
        Self::flush_mut(inner);
        &mut inner.0
    }

    fn eq_str(&self, s: &str) -> bool {
        self.borrow().deref().eq(s)
    }
}

impl Debug for JumpRopeBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let inner = self.0.borrow();
        f.debug_struct("BufferedRope")
            // .field("rope", &inner.0)
            .field("op", &inner.1)
            .finish()
    }
}

impl Clone for JumpRopeBuf {
    fn clone(&self) -> Self {
        let inner = self.0.borrow();
        Self(RefCell::new((inner.0.clone(), inner.1.clone())))
    }
}

impl<S: AsRef<str>> From<S> for JumpRopeBuf {
    fn from(str: S) -> Self {
        JumpRopeBuf::new_from_str(str.as_ref())
    }
}

impl<T: AsRef<str>> PartialEq<T> for JumpRopeBuf {
    fn eq(&self, other: &T) -> bool {
        self.eq_str(other.as_ref())
    }
}

// Needed for assert_eq!(&rope, "Hi there");
impl PartialEq<str> for JumpRopeBuf {
    fn eq(&self, other: &str) -> bool {
        self.eq_str(other)
    }
}

// Needed for assert_eq!(&rope, String::from("Hi there"));
impl PartialEq<String> for &JumpRopeBuf {
    fn eq(&self, other: &String) -> bool {
        self.eq_str(other.as_str())
    }
}