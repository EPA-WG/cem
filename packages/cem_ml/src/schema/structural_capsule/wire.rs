//! Bounded binary shape admission before typed decoding.
use super::*;

/// Allocation-free MessagePack shape preflight. Claimed collection/string
/// lengths are checked against remaining bytes before serde can reserve them.
pub(super) fn preflight(bytes: &[u8], control: &Work<'_>) -> Result<(), String> {
    let mut at = 0usize;
    let mut pending = vec![1usize];
    let mut work = 0;
    while let Some(left) = pending.last_mut() {
        if *left == 0 {
            pending.pop();
            continue;
        }
        *left -= 1;
        work += 1;
        check(control)?;
        if work > MAX_WORK || pending.len() > 64 {
            return Err(INVALID.into());
        }
        let tag = *bytes.get(at).ok_or(INVALID)?;
        at += 1;
        let mut read = |count: usize| -> Result<usize, String> {
            let end = at
                .checked_add(count)
                .filter(|n| *n <= bytes.len())
                .ok_or(INVALID)?;
            let value = bytes[at..end]
                .iter()
                .fold(0usize, |n, b| (n << 8) | *b as usize);
            at = end;
            Ok(value)
        };
        let (skip, items) = match tag {
            0x00..=0x7f | 0xe0..=0xff | 0xc0 | 0xc2 | 0xc3 => (0, 0),
            0x80..=0x8f => (0, 2 * (tag & 15) as usize),
            0x90..=0x9f => (0, (tag & 15) as usize),
            0xa0..=0xbf => ((tag & 31) as usize, 0),
            0xc4 | 0xd9 => (read(1)?, 0),
            0xc5 | 0xda => (read(2)?, 0),
            0xc6 | 0xdb => (read(4)?, 0),
            0xcc | 0xd0 => (1, 0),
            0xcd | 0xd1 => (2, 0),
            0xce | 0xd2 | 0xca => (4, 0),
            0xcf | 0xd3 | 0xcb => (8, 0),
            0xdc => (0, read(2)?),
            0xdd => (0, read(4)?),
            0xde => (0, read(2)?.checked_mul(2).ok_or(INVALID)?),
            0xdf => (0, read(4)?.checked_mul(2).ok_or(INVALID)?),
            _ => return Err(INVALID.into()),
        };
        at = at
            .checked_add(skip)
            .filter(|n| *n <= bytes.len())
            .ok_or(INVALID)?;
        if items > 2 * MAX_RECORDS || items > bytes.len() - at {
            return Err(INVALID.into());
        }
        if items > 0 {
            pending.push(items);
        }
    }
    if at != bytes.len() {
        return Err(INVALID.into());
    }
    Ok(())
}
