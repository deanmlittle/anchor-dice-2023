use anchor_lang::prelude::*;
use solana_program::{program_error::ProgramError};

pub const PUBKEY_SERIALIZED_SIZE: usize = 32;
pub const SIGNATURE_SERIALIZED_SIZE: usize = 64;
pub const SIGNATURE_OFFSETS_SERIALIZED_SIZE: usize = 14;
pub const SIGNATURE_OFFSETS_START: usize = 2;
pub const DATA_START: usize = SIGNATURE_OFFSETS_SERIALIZED_SIZE + SIGNATURE_OFFSETS_START;
const MARKER_BYTES: [u8;2] = [0xff, 0xff];

#[derive(Clone, Debug)]
pub struct Ed25519InstructionSignatures(pub Vec<Ed25519InstructionSignature>);

impl Ed25519InstructionSignatures {
    pub fn unpack(data: &[u8]) -> Result<Self> {
        if data.len() < SIGNATURE_OFFSETS_START {
            return Err(ProgramError::InvalidInstructionData.into())
        }
        let num_signatures = data[0] as usize;
        if num_signatures == 0 && data.len() > SIGNATURE_OFFSETS_START {
            return Err(ProgramError::InvalidInstructionData.into());
        }
        // Makre sure data is at least 16 bytes in length
        let expected_data_size = num_signatures
            .saturating_mul(SIGNATURE_OFFSETS_SERIALIZED_SIZE)
            .saturating_add(SIGNATURE_OFFSETS_START);
        if data.len() < expected_data_size {
            return Err(ProgramError::InvalidInstructionData.into());
        }
        let signatures: Vec<Ed25519InstructionSignature> = (0..num_signatures).into_iter().map(|i| {
            // Set the start and end positions of each signature to validate
            let start = i
                .saturating_mul(SIGNATURE_OFFSETS_SERIALIZED_SIZE)
                .saturating_add(SIGNATURE_OFFSETS_START);
            let end = start.saturating_add(SIGNATURE_OFFSETS_SERIALIZED_SIZE);
            // Unpack the signature
            let offsets = Ed25519InstructionOffsets::unpack(&data[start..end])?;
            let mut is_verifiable = true;
            let public_key: Option<Pubkey> = match offsets.public_key_instruction_index == u16::MAX {
                true => {
                    match Pubkey::try_from(
                        data[
                            offsets.public_key_offset as usize..offsets.public_key_offset as usize+PUBKEY_SERIALIZED_SIZE as usize
                        ].to_vec()
                    ) {
                        Ok(it) => Some(it),
                        Err(_) => return Err(ProgramError::InvalidInstructionData.into())
                    }
                },
                false => {
                    is_verifiable = false;
                    None
                }
            };
            let signature: Option<[u8;64]> = match offsets.signature_instruction_index == u16::MAX {
                true => {
                    let mut sig_buffer = [0u8; 64];
                    let start = offsets.signature_offset as usize;
                    sig_buffer.copy_from_slice(&data[start..start + SIGNATURE_SERIALIZED_SIZE]);
                    Some(sig_buffer)
                },
                false => {
                    is_verifiable = false;
                    None
                }
            };
            let message: Option<Vec<u8>> = match offsets.message_instruction_index == u16::MAX {
                true => {
                    Some(data[offsets.message_data_offset as usize..offsets.message_data_offset as usize+offsets.message_data_size as usize].to_vec())
                },
                false => {
                    is_verifiable = false;
                    None
                }
            };
            Ok(Ed25519InstructionSignature {
                is_verifiable,
                offsets,
                public_key,
                signature,
                message
            })
        }).collect::<Result<Vec<Ed25519InstructionSignature>>>()?;
        Ok(Ed25519InstructionSignatures(signatures))
    }
}

#[derive(Clone, Debug)]
pub struct Ed25519InstructionSignature {
    pub is_verifiable: bool,
    pub offsets: Ed25519InstructionOffsets,
    pub public_key: Option<Pubkey>,
    pub signature: Option<[u8;SIGNATURE_SERIALIZED_SIZE]>,
    pub message: Option<Vec<u8>>
}

#[derive(Clone, Copy, Debug)]
pub struct Ed25519InstructionOffsets {
    pub signature_offset: u16,
    pub signature_instruction_index: u16,
    pub public_key_offset: u16,
    pub public_key_instruction_index: u16,
    pub message_data_offset: u16,
    pub message_data_size: u16,
    pub message_instruction_index: u16,
}

impl Ed25519InstructionOffsets {
    pub fn new(message: &[u8]) -> Self {
        Self { 
            signature_offset: 48,
            signature_instruction_index: 0xffff,
            public_key_offset: 16,
            public_key_instruction_index: 0xffff,
            message_data_offset: 112,
            message_data_size: message.len() as u16,
            message_instruction_index: 0xffff
        }
    }

    pub fn pack(&self) -> [u8;14] {
        let mut s = [0u8; 14];
        s[0..2].copy_from_slice(&self.signature_offset.to_le_bytes());
        s[2..4].copy_from_slice(&MARKER_BYTES);
        s[4..6].copy_from_slice(&self.public_key_offset.to_le_bytes());
        s[6..8].copy_from_slice(&MARKER_BYTES);
        s[8..10].copy_from_slice(&self.message_data_offset.to_le_bytes());
        s[10..12].copy_from_slice(&self.message_data_size.to_le_bytes());
        s[12..14].copy_from_slice(&MARKER_BYTES);
        s
    }

    pub fn unpack(b: &[u8]) -> Result<Self> {
        if b.len() != 14 {
            return Err(ProgramError::InvalidInstructionData.into());
        }
        Ok(Self {
            signature_offset: u16::from_le_bytes([b[0], b[1]]),
            signature_instruction_index: u16::from_le_bytes([b[2], b[3]]),
            public_key_offset: u16::from_le_bytes([b[4], b[5]]),
            public_key_instruction_index: u16::from_le_bytes([b[6], b[7]]),
            message_data_offset: u16::from_le_bytes([b[8],b[9]]),
            message_data_size: u16::from_le_bytes([b[10], b[11]]),
            message_instruction_index: u16::from_le_bytes([b[12], b[13]])
        })
    }
}