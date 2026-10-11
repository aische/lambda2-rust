//! Deep recursion without stack overflows. The parser and the substitution functions recurse once
//! per nesting level; wrapping each recursive function in [`grow`] moves the rest of the recursion
//! onto a fresh heap-allocated stack segment whenever the current one runs low.

/// Grow when less than this much stack is left. Must cover one frame of any wrapped function.
const RED_ZONE: usize = 128 * 1024;
/// Size of each new stack segment.
const SEGMENT: usize = 4 * 1024 * 1024;

#[inline]
pub fn grow<R>(f: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(RED_ZONE, SEGMENT, f)
}
