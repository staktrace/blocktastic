use crate::{BlockParseError, BlockValidationError, LittleEndianSerialization, Opcode, Script, ScriptError};
use crate::parse::{read_bytes, IntoUsize};

impl LittleEndianSerialization for Opcode {
    fn serialize_le(&self, _dest: &mut Vec<u8>) {
        unimplemented!("Will implement this once I have script validation done to lock down the Opcode enum");
    }

    fn deserialize_le(bytes: &[u8], ix: &mut usize) -> Result<Self, BlockParseError> where Self: Sized {
        match u8::deserialize_le(bytes, ix)? {
            v @ 0x00..=0x4b => Ok(Opcode::PushArray(read_bytes(bytes, ix, v.usize()?)?)),
            0x4c => {
                let count = u8::deserialize_le(bytes, ix)?.usize()?;
                Ok(Opcode::PushArray(read_bytes(bytes, ix, count)?))
            }
            0x4d => {
                let count = u16::deserialize_le(bytes, ix)?.usize()?;
                Ok(Opcode::PushArray(read_bytes(bytes, ix, count)?))
            }
            0x4e => {
                let count = u32::deserialize_le(bytes, ix)?.usize()?;
                Ok(Opcode::PushArray(read_bytes(bytes, ix, count)?))
            }
            v @ 0x4f => Ok(Opcode::PushNumber(v as i8 - 0x50)),
            v @ 0x50 => Ok(Opcode::Reserved(v)),
            v @ 0x51..=0x60 => Ok(Opcode::PushNumber(v as i8 - 0x50)),
            v @ 0x61 => Ok(Opcode::Nop(v)),
            0x62 => Ok(Opcode::Ver),
            0x63 => Ok(Opcode::If),
            0x64 => Ok(Opcode::NotIf),
            0x65 => Ok(Opcode::VerIf),
            0x66 => Ok(Opcode::VerNotIf),
            0x67 => Ok(Opcode::Else),
            0x68 => Ok(Opcode::EndIf),
            0x69 => Ok(Opcode::Verify),
            0x6a => Ok(Opcode::Return),
            0x6b => Ok(Opcode::ToAltStack),
            0x6c => Ok(Opcode::FromAltStack),
            0x6d => Ok(Opcode::Drop2),
            0x6e => Ok(Opcode::Dup2),
            0x6f => Ok(Opcode::Dup3),
            0x70 => Ok(Opcode::Over2),
            0x71 => Ok(Opcode::Rot2),
            0x72 => Ok(Opcode::Swap2),
            0x73 => Ok(Opcode::IfDup),
            0x74 => Ok(Opcode::Depth),
            0x75 => Ok(Opcode::Drop),
            0x76 => Ok(Opcode::Dup),
            0x77 => Ok(Opcode::Nip),
            0x78 => Ok(Opcode::Over),
            0x79 => Ok(Opcode::Pick),
            0x7a => Ok(Opcode::Roll),
            0x7b => Ok(Opcode::Rot),
            0x7c => Ok(Opcode::Swap),
            0x7d => Ok(Opcode::Tuck),
            v @ 0x7e..=0x81 => Ok(Opcode::Disabled(v)),
            0x82 => Ok(Opcode::Size),
            v @ 0x83..=0x86 => Ok(Opcode::Disabled(v)),
            0x87 => Ok(Opcode::Equal),
            0x88 => Ok(Opcode::EqualVerify),
            v @ 0x89..=0x8a => Ok(Opcode::Reserved(v)),
            0x8b => Ok(Opcode::Add1),
            0x8c => Ok(Opcode::Sub1),
            v @ 0x8d..=0x8e => Ok(Opcode::Disabled(v)),
            0x8f => Ok(Opcode::Negate),
            0x90 => Ok(Opcode::Abs),
            0x91 => Ok(Opcode::Not),
            0x92 => Ok(Opcode::NotEqual0),
            0x93 => Ok(Opcode::Add),
            0x94 => Ok(Opcode::Sub),
            v @ 0x95..=0x99 => Ok(Opcode::Disabled(v)),
            0x9a => Ok(Opcode::BoolAnd),
            0x9b => Ok(Opcode::BoolOr),
            0x9c => Ok(Opcode::NumEqual),
            0x9d => Ok(Opcode::NumEqualVerify),
            0x9e => Ok(Opcode::NumNotEqual),
            0x9f => Ok(Opcode::LessThan),
            0xa0 => Ok(Opcode::GreaterThan),
            0xa1 => Ok(Opcode::LessThanOrEqual),
            0xa2 => Ok(Opcode::GreaterThanOrEqual),
            0xa3 => Ok(Opcode::Min),
            0xa4 => Ok(Opcode::Max),
            0xa5 => Ok(Opcode::Within),
            0xa6 => Ok(Opcode::RIPEMD160),
            0xa7 => Ok(Opcode::SHA1),
            0xa8 => Ok(Opcode::SHA256),
            0xa9 => Ok(Opcode::Hash160),
            0xaa => Ok(Opcode::Hash256),
            0xab => Ok(Opcode::CodeSeparator),
            0xac => Ok(Opcode::CheckSig),
            0xad => Ok(Opcode::CheckSigVerify),
            0xae => Ok(Opcode::CheckMultisig),
            0xaf => Ok(Opcode::CheckMultisigVerify),
            v @ 0xb0 => Ok(Opcode::Nop(v)),
            0xb1 => Ok(Opcode::CheckLockTimeVerify),
            0xb2 => Ok(Opcode::CheckSequenceVerify),
            v @ 0xb3..=0xb9 => Ok(Opcode::Nop(v)),
            v @ 0xba..=0xff => Ok(Opcode::Invalid(v)),
        }
    }
}

pub fn parse_script(bytes: &[u8]) -> Result<Script, BlockParseError> {
    let mut opcodes = Vec::new();

    let mut ix = 0;
    while ix < bytes.len() {
        opcodes.push(Opcode::deserialize_le(bytes, &mut ix)?);
    }
    assert!(ix == bytes.len(), "The last call to read_opcode should have returned an error");
    Ok(Script {
        opcodes,
    })
}

enum StackEntry {
    Bytes(Vec<u8>),
    Number(i64),
}

struct Executor {
    stack: Vec<StackEntry>,
}

impl Executor {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
        }
    }

    fn execute(&mut self, script: Script) -> Result<(), BlockValidationError> {
        for opcode in script.opcodes {
            match opcode {
                Opcode::PushArray(v) => self.stack.push(StackEntry::Bytes(v)),
                Opcode::PushNumber(v) => self.stack.push(StackEntry::Number(v.into())),
                Opcode::Reserved(op) => return Err(BlockValidationError::new(format!("Unexpected reserved opcode {}", op))),
                Opcode::Nop(_) => (),
                _ => (),

/*
    Opcode::Ver, // 0x62
    Opcode::If, // 0x63
    Opcode::NotIf, // 0x64
    Opcode::VerIf, // 0x65
    Opcode::VerNotIf, // 0x66
    Opcode::Else, // 0x67
    Opcode::EndIf, // 0x68
    Opcode::Verify, // 0x69
    Opcode::Return, // 0x6a

    Opcode::ToAltStack, // 0x6b
    Opcode::FromAltStack, // 0x6c
    Opcode::Drop2, // 0x6d
    Opcode::Dup2, // 0x6e
    Opcode::Dup3, // 0x6f
    Opcode::Over2, // 0x70
    Opcode::Rot2, // 0x71
    Opcode::Swap2, // 0x72
    Opcode::IfDup, // 0x73
    Opcode::Depth, // 0x74
    Opcode::Drop, // 0x75
    Opcode::Dup, // 0x76
    Opcode::Nip, // 0x77
    Opcode::Over, // 0x78
    Opcode::Pick, // 0x79
    Opcode::Roll, // 0x7a
    Opcode::Rot, // 0x7b
    Opcode::Swap, // 0x7c
    Opcode::Tuck, // 0x7d

    Opcode::Disabled(u8)
    Opcode::Size, // 0x82

    Opcode::Equal, // 0x87
    Opcode::EqualVerify, // 0x88

    Opcode::Add1, // 0x8b
    Opcode::Sub1, // 0x8c
    Opcode::Negate, // 0x8f
    Opcode::Abs, // 0x90
    Opcode::Not, // 0x91
    Opcode::NotEqual0, // 0x92
    Opcode::Add, // 0x93
    Opcode::Sub, // 0x94

    Opcode::BoolAnd, // 0x9a
    Opcode::BoolOr, // 0x9b
    Opcode::NumEqual, // 0x9c
    Opcode::NumEqualVerify, // 0x9d
    Opcode::NumNotEqual, // 0x9e
    Opcode::LessThan, // 0x9f
    Opcode::GreaterThan, // 0xa0
    Opcode::LessThanOrEqual, // 0xa1
    Opcode::GreaterThanOrEqual, // 0xa2
    Opcode::Min, // 0xa3
    Opcode::Max, // 0xa4
    Opcode::Within, // 0xa5

    Opcode::RIPEMD160, // 0xa6
    Opcode::SHA1, // 0xa7
    Opcode::SHA256, // 0xa8
    Opcode::Hash160, // 0xa9
    Opcode::Hash256, // 0xaa
    Opcode::CodeSeparator, // 0xab
    Opcode::CheckSig, // 0xac
    Opcode::CheckSigVerify, // 0xad
    Opcode::CheckMultisig, // 0xae
    Opcode::CheckMultisigVerify, // 0xaf

    Opcode::CheckLockTimeVerify, // 0xb1
    Opcode::CheckSequenceVerify, // 0xb2

    Opcode::Invalid(u8), // 0xba - 0xff
*/
            }
        }
        Ok(())
    }
}

#[allow(unused)]
pub fn verify(lock: &[u8], unlock: &[u8]) -> Result<bool, ScriptError> {
    let lock = parse_script(lock).map_err(ScriptError::Parse)?;
    let unlock = parse_script(unlock).map_err(ScriptError::Parse)?;
    let mut executor = Executor::new();
    executor.execute(unlock).map_err(ScriptError::Validation)?;
    executor.execute(lock).map_err(ScriptError::Validation)?;
    Ok(true)
}
