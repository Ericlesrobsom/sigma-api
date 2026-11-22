//! Solver do CAP (proof-of-work) usado no login do painel.
//!
//! Formato 1 do Cap: o desafio é derivado do token JWT por FNV-1a + PRNG
//! xorshift; a solução é o menor nonce cujo SHA-256(salt + nonce) começa
//! pelo prefixo alvo. Implementação conferida contra o capjs-core.

use sha2::{Digest, Sha256};

/// FNV-1a de 32 bits, sobre code units UTF-16 (igual ao JS).
pub fn fnv1a(s: &str) -> u32 {
    let mut hash: u32 = 2_166_136_261;
    for unit in s.encode_utf16() {
        hash ^= unit as u32;
        hash = hash.wrapping_add(mix(hash));
    }
    hash
}

/// Continua um FNV-1a já iniciado.
pub fn fnv1a_resume(state: u32, s: &str) -> u32 {
    let mut hash = state;
    for unit in s.encode_utf16() {
        hash ^= unit as u32;
        hash = hash.wrapping_add(mix(hash));
    }
    hash
}

fn mix(hash: u32) -> u32 {
    hash.wrapping_shl(1)
        .wrapping_add(hash.wrapping_shl(4))
        .wrapping_add(hash.wrapping_shl(7))
        .wrapping_add(hash.wrapping_shl(8))
        .wrapping_add(hash.wrapping_shl(24))
}

/// PRNG xorshift32 usado pelo Cap para derivar salt/alvo.
pub fn prng_from_hash(initial: u32, length: usize) -> String {
    let mut state = initial;
    let mut out = String::with_capacity(length + 8);
    while out.len() < length {
        state ^= state.wrapping_shl(13);
        state ^= state >> 17;
        state ^= state.wrapping_shl(5);
        out.push_str(&format!("{state:08x}"));
    }
    out.truncate(length);
    out
}

/// Resolve todos os desafios do CAP e devolve as soluções na ordem.
pub fn solve(token: &str, c: usize, s: usize, d: usize) -> Vec<u64> {
    let token_fnv = fnv1a(token);
    (0..c)
        .map(|i| {
            let salt_seed = fnv1a_resume(token_fnv, &(i + 1).to_string());
            let target_seed = fnv1a_resume(salt_seed, "d");
            let salt = prng_from_hash(salt_seed, s);
            let target = prng_from_hash(target_seed, d);
            solve_one(&salt, &target)
        })
        .collect()
}

/// Confere uma solução já existente (usado nos testes com vetor real).
pub fn verify_solution(token: &str, index: usize, s: usize, d: usize, solution: u64) -> bool {
    let salt_seed = fnv1a_resume(fnv1a(token), &(index + 1).to_string());
    let target_seed = fnv1a_resume(salt_seed, "d");
    let salt = prng_from_hash(salt_seed, s);
    let target = prng_from_hash(target_seed, d);
    hash_matches(&salt, solution, &target)
}

fn solve_one(salt: &str, target: &str) -> u64 {
    let mut buf = itoa::Buffer::new();
    for nonce in 0u64.. {
        let mut hasher = Sha256::new();
        hasher.update(salt.as_bytes());
        hasher.update(buf.format(nonce).as_bytes());
        let digest = hasher.finalize();
        if prefix_matches(&digest, target) {
            return nonce;
        }
    }
    unreachable!("nonce u64 esgotado")
}

fn hash_matches(salt: &str, solution: u64, target: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(solution.to_string().as_bytes());
    let digest = hasher.finalize();
    prefix_matches(&digest, target)
}

fn prefix_matches(digest: &[u8], target: &str) -> bool {
    let bytes = target.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let hi = hex_val(bytes[i]);
        let lo = hex_val(bytes[i + 1]);
        let byte = digest[i / 2];
        if (byte >> 4) != hi || (byte & 0x0f) != lo {
            return false;
        }
        i += 2;
    }
    if i < bytes.len() {
        return (digest[i / 2] >> 4) == hex_val(bytes[i]);
    }
    true
}

fn hex_val(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0xff,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_conhecido() {
        assert_eq!(fnv1a(""), 2_166_136_261);
        assert_eq!(fnv1a("a"), 0xe40c292c);
    }

    #[test]
    fn prng_deterministico() {
        assert_eq!(prng_from_hash(1, 8), "00042021");
    }
}
