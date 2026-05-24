//! Binary artifact to IR deserialization shell.

use crate::ir::lower::IrLowerer;
use crate::ir::serialize::IrSerializer;
use crate::ir::CompiledQuery;
use crate::parser::Parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrDeserializer;

impl IrDeserializer {
    pub fn deserialize(bytes: &[u8]) -> Result<CompiledQuery, String> {
        let mut reader = Reader::new(bytes);
        reader.read_magic(IrSerializer::MAGIC)?;
        let source = reader.read_string()?;
        let policy_count = reader.read_u32()? as usize;
        let mut policy_names = Vec::with_capacity(policy_count);
        for _ in 0..policy_count {
            policy_names.push(reader.read_string()?);
        }
        if !reader.is_empty() {
            return Err("compiled artifact has trailing bytes".to_owned());
        }

        let parsed = Parser::new(&source).parse_module();
        if let Some(diagnostic) = parsed
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity.is_hard_violation())
        {
            return Err(format!("artifact parse failed: {}", diagnostic.message));
        }
        let lowered = IrLowerer::new()
            .with_policy_bindings(policy_names)
            .lower_module(&parsed.module);
        if let Some(diagnostic) = lowered
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity.is_hard_violation())
        {
            return Err(format!("artifact lower failed: {}", diagnostic.message));
        }
        Ok(lowered.query)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn read_magic(&mut self, magic: &[u8]) -> Result<(), String> {
        let actual = self.read_bytes(magic.len())?;
        if actual == magic {
            Ok(())
        } else {
            Err("compiled artifact magic mismatch".to_owned())
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| "compiled artifact string is not valid UTF-8".to_owned())
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes = self.read_bytes(4)?;
        let mut raw = [0u8; 4];
        raw.copy_from_slice(bytes);
        Ok(u32::from_le_bytes(raw))
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .cursor
            .checked_add(len)
            .ok_or_else(|| "compiled artifact length overflow".to_owned())?;
        if end > self.bytes.len() {
            return Err("compiled artifact ended unexpectedly".to_owned());
        }
        let out = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(out)
    }

    fn is_empty(&self) -> bool {
        self.cursor == self.bytes.len()
    }
}
