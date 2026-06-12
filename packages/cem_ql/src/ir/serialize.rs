//! IR to binary artifact serialization shell.

use crate::ir::CompiledQuery;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrSerializer;

impl IrSerializer {
    pub const MAGIC: &'static [u8] = b"CEMQLIR1\n";

    pub fn serialize(query: &CompiledQuery) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(Self::MAGIC);
        write_string(&mut out, &query.source);

        let mut policy_names = query.policy_bindings.values().cloned().collect::<Vec<_>>();
        policy_names.sort();
        policy_names.dedup();
        write_u32(&mut out, policy_names.len());
        for name in policy_names {
            write_string(&mut out, &name);
        }
        out
    }
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_u32(out, value.len());
    out.extend_from_slice(value.as_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&(value.min(u32::MAX as usize) as u32).to_le_bytes());
}
