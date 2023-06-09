use ark_ff::{BitIteratorBE, FpParameters, PrimeField};

/// Converts the provided big endian bits to LE bytes
pub fn bits_be_to_bytes_le(bits: &[bool]) -> Vec<u8> {
    let reversed_bits = {
        let mut tmp = bits.to_owned();
        tmp.reverse();
        tmp
    };

    let mut bytes = vec![];
    for chunk in reversed_bits.chunks(8) {
        let mut byte = 0;
        let mut twoi: u64 = 1;
        for c in chunk {
            byte += (twoi * (*c as u64)) as u8;
            twoi *= 2;
        }
        bytes.push(byte);
    }

    bytes
}

/// Converts the provided little endian bits to LE bytes
pub fn bits_le_to_bytes_le(bits: &[bool]) -> Vec<u8> {
    bits_be_to_bytes_le(&bits.iter().cloned().rev().collect::<Vec<_>>())
}

/// If bytes is a little endian representation of a number, this returns the bits
/// of the number in descending order
pub fn bytes_le_to_bits_be(bytes: &[u8], bits_to_take: usize) -> Vec<bool> {
    let mut bits = vec![];
    for b in bytes {
        let mut byte = *b;
        for _ in 0..8 {
            bits.push((byte & 1) == 1);
            byte >>= 1;
        }
    }

    bits.into_iter()
        .take(bits_to_take)
        .collect::<Vec<bool>>()
        .into_iter()
        .rev()
        .collect()
}

/// Converts the provided little endian bytes to LE bits
pub fn bytes_le_to_bits_le(bytes: &[u8], bits_to_take: usize) -> Vec<bool> {
    bytes_le_to_bits_be(bytes, bits_to_take)
        .into_iter()
        .rev()
        .collect()
}

pub fn to_bits_le<F: PrimeField>(input: F) -> Vec<bool> {
    let field_characteristic = BitIteratorBE::new(F::characteristic());
    let mut bits: Vec<_> = BitIteratorBE::new(input.into_repr())
        .zip(field_characteristic)
        .skip_while(|(_, c)| !c)
        .map(|(b, _)| b)
        .collect();
    assert_eq!(bits.len(), F::Params::MODULUS_BITS as usize);
    bits.reverse();

    bits
}

pub fn to_bytes_le<F: PrimeField>(input: F) -> Vec<u8> {
    bits_le_to_bytes_le(&to_bits_le(input))
}
