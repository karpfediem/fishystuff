use anyhow::{bail, Result};

pub fn decompress_bdo(input: &[u8]) -> Result<Vec<u8>> {
    if input.len() < 3 {
        bail!("compressed payload is too short");
    }

    let output_len = if input[0] & 0x02 != 0 {
        read_u32(input, 5)? as usize
    } else {
        input[2] as usize
    };

    let mut output = vec![0u8; output_len];
    if input[0] & 0x01 != 0 {
        let written = blackdesert_unpack_core(input, &mut output)?;
        if written != output_len {
            output.truncate(written);
        }
    } else {
        let header_len = if input[0] & 0x02 != 0 { 9 } else { 3 };
        let end = header_len + output_len;
        if input.len() < end {
            bail!("compressed payload header is truncated");
        }
        output.copy_from_slice(&input[header_len..end]);
    }

    Ok(output)
}

fn blackdesert_unpack_core(input: &[u8], output: &mut [u8]) -> Result<usize> {
    const DATA_LENGTH_TABLE: [usize; 16] = [4, 0, 1, 0, 2, 0, 1, 0, 3, 0, 1, 0, 2, 0, 1, 0];

    let mut out_idx = 0usize;
    let mut block_group_header = 1u32;

    let (compressed_length, mut in_idx) = if input[0] & 0x02 != 0 {
        (read_u32(input, 1)? as usize, 9usize)
    } else {
        (input[1] as usize, 3usize)
    };

    if compressed_length == 0 || compressed_length > input.len() {
        bail!("compressed payload length is invalid");
    }

    let last_input_idx = compressed_length - 1;
    let Some(last_output_idx) = output.len().checked_sub(1) else {
        return Ok(0);
    };

    loop {
        loop {
            if block_group_header == 1 {
                if in_idx + 3 > last_input_idx {
                    bail!("truncated BDO block group header");
                }
                block_group_header = read_u32(input, in_idx)?;
                in_idx += 4;
            }

            if in_idx + 3 > last_input_idx {
                bail!("truncated BDO block header");
            }

            let block_header = read_u32(input, in_idx)?;
            if block_group_header & 1 == 0 {
                if out_idx >= last_output_idx.saturating_sub(10) {
                    break;
                }

                let valid_data_length = DATA_LENGTH_TABLE[(block_group_header & 0xF) as usize];
                let end = out_idx + 4;
                if end > output.len() {
                    bail!("decompression would write past the output buffer");
                }
                output[out_idx..end].copy_from_slice(&block_header.to_le_bytes());
                block_group_header >>= valid_data_length as u32;
                out_idx += valid_data_length;
                in_idx += valid_data_length;
                continue;
            }

            let (repeat_index, block_length, header_len) = if block_header & 0x03 == 0x03 {
                if block_header & 0x7F == 3 {
                    (
                        (block_header >> 15) as usize,
                        (((block_header >> 7) & 0xFF) + 3) as usize,
                        4usize,
                    )
                } else {
                    (
                        ((block_header >> 7) & 0x1FFFF) as usize,
                        (((block_header >> 2) & 0x1F) + 2) as usize,
                        3usize,
                    )
                }
            } else if block_header & 0x03 == 0x02 {
                (
                    (((block_header as u16) >> 6) as usize),
                    (((block_header >> 2) & 0xF) + 3) as usize,
                    2usize,
                )
            } else if block_header & 0x03 == 0x01 {
                ((((block_header as u16) >> 2) as usize), 3usize, 2usize)
            } else {
                ((((block_header as u8) >> 2) as usize), 3usize, 1usize)
            };

            in_idx += header_len;
            if repeat_index < 3 || out_idx < repeat_index {
                bail!("corrupted BDO back-reference");
            }
            if block_length > output.len().saturating_sub(out_idx + 4) {
                bail!("BDO block length exceeds the output buffer");
            }

            let mut ptr = out_idx;
            for _ in (0..block_length).step_by(3) {
                if ptr + 4 > output.len() || ptr < repeat_index {
                    bail!("corrupted BDO back-reference copy");
                }
                let src = ptr - repeat_index;
                let word = [
                    output[src],
                    output[src + 1],
                    output[src + 2],
                    output[src + 3],
                ];
                output[ptr..ptr + 4].copy_from_slice(&word);
                ptr += 3;
            }

            block_group_header >>= 1;
            out_idx += block_length;
        }

        if out_idx >= last_output_idx.saturating_sub(10) {
            break;
        }
    }

    if out_idx <= last_output_idx {
        let end_input = last_input_idx + 1;
        loop {
            if block_group_header == 1 {
                in_idx += 4;
                block_group_header = 0x8000_0000;
            }

            if in_idx >= end_input {
                break;
            }

            output[out_idx] = input[in_idx];
            out_idx += 1;
            in_idx += 1;
            block_group_header >>= 1;

            if out_idx > last_output_idx {
                return Ok(out_idx);
            }
        }

        bail!("decompressed data are larger than expected");
    }

    Ok(out_idx)
}

fn read_u32(buffer: &[u8], offset: usize) -> Result<u32> {
    let bytes = buffer
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow::anyhow!("truncated u32 at offset {offset}"))?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::decompress_bdo;

    #[test]
    fn uncompressed_payload_round_trips() {
        let data = b"hello";
        let mut payload = Vec::new();
        payload.push(0x6E);
        payload.extend_from_slice(&(9u32 + data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&(data.len() as u32).to_le_bytes());
        payload.extend_from_slice(data);

        let output = decompress_bdo(&payload).unwrap();
        assert_eq!(output, data);
    }
}
