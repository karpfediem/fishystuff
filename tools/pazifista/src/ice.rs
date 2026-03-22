use std::sync::OnceLock;

use anyhow::{bail, Result};

#[derive(Clone, Copy, Default)]
struct IceSubkey {
    val: [u32; 3],
}

pub struct IceKey {
    rounds: usize,
    keysched: Vec<IceSubkey>,
}

impl IceKey {
    pub fn thin(key: [u8; 8]) -> Self {
        let mut ice = Self {
            rounds: 8,
            keysched: vec![IceSubkey::default(); 8],
        };
        ice.set(&key);
        ice
    }

    pub fn decrypt_buffer(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        if encrypted.len() % 8 != 0 {
            bail!("ICE input must be a multiple of 8 bytes");
        }

        let mut decrypted = vec![0u8; encrypted.len()];
        for (src, dst) in encrypted.chunks_exact(8).zip(decrypted.chunks_exact_mut(8)) {
            let mut block = [0u8; 8];
            block.copy_from_slice(src);
            dst.copy_from_slice(&self.decrypt_block(block));
        }

        Ok(decrypted)
    }

    fn decrypt_block(&self, ciphertext: [u8; 8]) -> [u8; 8] {
        let mut l = u32::from_be_bytes(ciphertext[0..4].try_into().unwrap());
        let mut r = u32::from_be_bytes(ciphertext[4..8].try_into().unwrap());

        for i in (1..self.rounds).rev().step_by(2) {
            l ^= ice_f(r, &self.keysched[i]);
            r ^= ice_f(l, &self.keysched[i - 1]);
        }

        let mut out = [0u8; 8];
        out[0..4].copy_from_slice(&r.to_be_bytes());
        out[4..8].copy_from_slice(&l.to_be_bytes());
        out
    }

    fn set(&mut self, key: &[u8; 8]) {
        let mut kb = [0u16; 4];
        for i in 0..4 {
            kb[3 - i] = ((key[i * 2] as u16) << 8) | key[i * 2 + 1] as u16;
        }
        schedule_build(&mut self.keysched, &mut kb, 0, &ICE_KEYROT);
    }
}

const ICE_SMOD: [[u32; 4]; 4] = [
    [333, 313, 505, 369],
    [379, 375, 319, 391],
    [361, 445, 451, 397],
    [397, 425, 395, 505],
];

const ICE_SXOR: [[u32; 4]; 4] = [
    [0x83, 0x85, 0x9B, 0xCD],
    [0xCC, 0xA7, 0xAD, 0x41],
    [0x4B, 0x2E, 0xD4, 0x33],
    [0xEA, 0xCB, 0x2E, 0x04],
];

const ICE_PBOX: [u32; 32] = [
    0x00000001, 0x00000080, 0x00000400, 0x00002000, 0x00080000, 0x00200000, 0x01000000, 0x40000000,
    0x00000008, 0x00000020, 0x00000100, 0x00004000, 0x00010000, 0x00800000, 0x04000000, 0x20000000,
    0x00000004, 0x00000010, 0x00000200, 0x00008000, 0x00020000, 0x00400000, 0x08000000, 0x10000000,
    0x00000002, 0x00000040, 0x00000800, 0x00001000, 0x00040000, 0x00100000, 0x02000000, 0x80000000,
];

const ICE_KEYROT: [usize; 16] = [0, 1, 2, 3, 2, 1, 3, 0, 1, 3, 2, 0, 3, 1, 0, 2];

fn ice_f(p: u32, sk: &IceSubkey) -> u32 {
    let sboxes = ice_sboxes();
    let tl = ((p >> 16) & 0x3ff) | (((p >> 14) | (p << 18)) & 0xffc00);
    let tr = (p & 0x3ff) | ((p << 2) & 0xffc00);

    let mut al = sk.val[2] & (tl ^ tr);
    let mut ar = al ^ tr;
    al ^= tl;

    al ^= sk.val[0];
    ar ^= sk.val[1];

    sboxes[0][(al >> 10) as usize]
        | sboxes[1][(al & 0x3ff) as usize]
        | sboxes[2][(ar >> 10) as usize]
        | sboxes[3][(ar & 0x3ff) as usize]
}

fn schedule_build(keysched: &mut [IceSubkey], kb: &mut [u16; 4], start: usize, keyrot: &[usize]) {
    for i in 0..8 {
        let kr = keyrot[i];
        let sk = &mut keysched[start + i];
        sk.val = [0; 3];

        for j in 0..15 {
            let curr_sk = &mut sk.val[j % 3];
            for k in 0..4 {
                let curr_kb = &mut kb[(kr + k) & 3];
                let bit = *curr_kb & 1;
                *curr_sk = (*curr_sk << 1) | u32::from(bit);
                *curr_kb = (*curr_kb >> 1) | ((bit ^ 1) << 15);
            }
        }
    }
}

fn ice_sboxes() -> &'static [[u32; 1024]; 4] {
    static ICE_SBOXES: OnceLock<[[u32; 1024]; 4]> = OnceLock::new();
    ICE_SBOXES.get_or_init(|| {
        let mut sboxes = [[0u32; 1024]; 4];
        for i in 0..1024usize {
            let col = ((i >> 1) & 0xff) as u32;
            let row = ((i & 0x1) | ((i & 0x200) >> 8)) as usize;

            let x0 = gf_exp7(col ^ ICE_SXOR[0][row], ICE_SMOD[0][row]) << 24;
            sboxes[0][i] = ice_perm32(x0);

            let x1 = gf_exp7(col ^ ICE_SXOR[1][row], ICE_SMOD[1][row]) << 16;
            sboxes[1][i] = ice_perm32(x1);

            let x2 = gf_exp7(col ^ ICE_SXOR[2][row], ICE_SMOD[2][row]) << 8;
            sboxes[2][i] = ice_perm32(x2);

            let x3 = gf_exp7(col ^ ICE_SXOR[3][row], ICE_SMOD[3][row]);
            sboxes[3][i] = ice_perm32(x3);
        }
        sboxes
    })
}

fn gf_mult(mut a: u32, mut b: u32, m: u32) -> u32 {
    let mut res = 0u32;
    while b != 0 {
        if b & 1 != 0 {
            res ^= a;
        }
        a <<= 1;
        b >>= 1;
        if a >= 256 {
            a ^= m;
        }
    }
    res
}

fn gf_exp7(b: u32, m: u32) -> u32 {
    if b == 0 {
        return 0;
    }
    let x = gf_mult(b, b, m);
    let x = gf_mult(b, x, m);
    let x = gf_mult(x, x, m);
    gf_mult(b, x, m)
}

fn ice_perm32(mut x: u32) -> u32 {
    let mut res = 0u32;
    let mut index = 0usize;
    while x != 0 {
        if x & 1 != 0 {
            res |= ICE_PBOX[index];
        }
        index += 1;
        x >>= 1;
    }
    res
}
