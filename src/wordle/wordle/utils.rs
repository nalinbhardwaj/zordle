use halo2_proofs::pasta::Fp;

pub const BASE: u64 = 29;
pub const WORD_COUNT: usize = 3;
pub const WORD_LEN : usize = 5;

pub fn word_to_chars(word: &str) -> Vec<u64> {
    let mut res = vec![];
    for c in word.chars() {
        res.push((c as u64) - ('a' as u64) + 1);
    }
    res
}

pub fn word_to_polyhash(word: &str) -> u64 {
    let chars = word_to_chars(word);
    let mut hash = 0;
    for c in chars {
        hash = hash * BASE;
        hash += c;
    }

    hash
}

pub fn compute_diff(word: &str, final_word: &str) -> Vec<Vec<Fp>> {
    let mut res = vec![];
    let mut green = vec![];
    for i in 0..WORD_LEN {
        if word.chars().nth(i) == final_word.chars().nth(i) {
            green.push(Fp::one());
        } else {
            green.push(Fp::zero());
        }
    }
    res.push(green);

    let mut yellow = vec![Fp::zero(); WORD_LEN];
    for i in 0..WORD_LEN {
        for j in 0..WORD_LEN {
            if word.chars().nth(i) == final_word.chars().nth(j) {
                yellow[i] = Fp::one();
            }
        }
    }
    res.push(yellow);
    
    res
}