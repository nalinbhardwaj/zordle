use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*, poly::Rotation};
use halo2_proofs::{circuit::Value, dev::MockProver, pasta::Fp};
use halo2_proofs::transcript::{Blake2bRead, Blake2bWrite, Challenge255, EncodedChallenge};
use halo2_proofs::plonk::{
    create_proof, keygen_pk, keygen_vk, verify_proof, Advice, Assigned, BatchVerifier, Circuit,
    Column, ConstraintSystem, Error, Fixed, SingleVerifier, TableColumn, VerificationStrategy,
};
use halo2_proofs::poly::{commitment::Params};
use halo2_proofs::pasta::{Eq, EqAffine};
use rand_core::OsRng;
use std::fs::File;
use std::io::{self, Write, Read, BufReader, BufWriter};
use wasm_bindgen::prelude::*;
use js_sys::Uint8Array;

use crate::wordle::wordle::{*, utils::*};

pub use wasm_bindgen_rayon::init_thread_pool;

const K: u32 = 14;

extern crate console_error_panic_hook;

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn verify_play(final_word: String, proof_js: JsValue, diffs_u64_js: JsValue, params_ser: JsValue) -> bool {
    let params_vec = params_ser.into_serde::<Vec<u8>>().unwrap();
    let proof = proof_js.into_serde::<Vec<u8>>().unwrap();
    let diffs_u64 = diffs_u64_js.into_serde::<[[[u64; WORD_LEN]; 2]; WORD_COUNT]>().unwrap();

    let final_chars = word_to_chars(&final_word);

    let empty_circuit = WordleCircuit::<Fp> {
        poly_words: [Value::unknown(); WORD_COUNT],
        word_chars: [[Value::unknown(); WORD_LEN]; WORD_COUNT],
        word_diffs_green: [[Value::unknown(); WORD_LEN]; WORD_COUNT],
        word_diffs_yellow: [[Value::unknown(); WORD_LEN]; WORD_COUNT],
    };


    let mut instance = Vec::new();

    // final word chars
    let mut final_chars_instance = vec![];
    for i in 0..WORD_LEN {
        final_chars_instance.push(Fp::from(final_chars[i]));
    }
    instance.push(final_chars_instance.clone());

    // read json diffs to array
    let mut diffs = vec![];

    for idx in 0..WORD_COUNT {
        let mut diff_instance = vec![];
        for i in 0..2 {
            let mut col_row = vec![];
            for j in 0..WORD_LEN {
                col_row.push(Fp::from(diffs_u64[idx][i][j]));
            }
            diff_instance.push(col_row);
        }
        diffs.push(diff_instance.clone());
    }

    // color green
    let mut green = vec![];
    for idx in 0..WORD_COUNT {
        for i in 0..WORD_LEN {
            green.push(diffs[idx][0][i]);
        }
    }
    instance.push(green.clone());

    // color yellow
    let mut yellow = vec![];
    for idx in 0..WORD_COUNT {
        for i in 0..WORD_LEN {
            yellow.push(diffs[idx][1][i]);
        }
    }
    instance.push(yellow.clone());

    let mut instance_slice = [
        &final_chars_instance.clone()[..],
        &green.clone()[..],
        &yellow.clone()[..],
    ];

    // TODO
    // let params_fs = File::open("params.bin").unwrap();
    // let params = Params::<EqAffine>::read(&mut BufReader::new(params_fs)).unwrap();
    let params = Params::<EqAffine>::read(&mut BufReader::new(&params_vec[..])).unwrap();

    let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
    // Check that a hardcoded proof is satisfied
    let strategy = SingleVerifier::new(&params);
    let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
    
    verify_proof(
        &params,
        &vk,
        strategy,
        &[&instance_slice, &instance_slice],
        &mut transcript,
    ).is_ok()
}

#[wasm_bindgen]
pub fn get_play_diff(final_word: String, words_js: JsValue) -> JsValue {
    let mut words = words_js.into_serde::<Vec<String>>().unwrap();
    let mut diffs_u64 = [[[0; WORD_LEN]; 2]; WORD_COUNT];
    for idx in 0..WORD_COUNT {
        let diff_u64 = compute_diff_u64(&words[idx], &final_word);
        for i in 0..2 {
            for j in 0..WORD_LEN {
                diffs_u64[idx][i][j] = diff_u64[i][j];
            }
        }
    }

    JsValue::from_serde(&diffs_u64).unwrap()
}

#[wasm_bindgen]
pub async fn prove_play(final_word: String, words_js: JsValue, params_ser: JsValue) -> JsValue {
    let mut words = words_js.into_serde::<Vec<String>>().unwrap();
    let params_vec = Uint8Array::new(&params_ser).to_vec();

    let mut poly_words: [Value<Assigned<Fp>>; WORD_COUNT] = [Value::known(Fp::from(123).into()); WORD_COUNT];
    let mut word_chars: [[Value<Assigned<Fp>>; WORD_LEN]; WORD_COUNT] = [[Value::known(Fp::from(123).into()); WORD_LEN]; WORD_COUNT];

    for idx in 0..WORD_COUNT {
        poly_words[idx] = Value::known(Fp::from(word_to_polyhash(&words[idx].clone())).into());
        let chars = word_to_chars(&words[idx].clone());
        for i in 0..WORD_LEN {
            word_chars[idx][i] = Value::known(Fp::from(chars[i]).into());
        }
    }

    let final_chars = word_to_chars(&final_word);

    let mut word_diffs_green = [[Value::known(Fp::from(123).into()); WORD_LEN]; WORD_COUNT];
    let mut word_diffs_yellow = [[Value::known(Fp::from(123).into()); WORD_LEN]; WORD_COUNT];
    for idx in 0..WORD_COUNT {
        let chars = word_to_chars(&words[idx].clone());
        for i in 0..WORD_LEN {
            word_diffs_green[idx][i] = Value::known((Fp::from(chars[i]) - Fp::from(final_chars[i])).into());
        }

        for i in 0..WORD_LEN {
            let yellow_diff = {
                (0..WORD_LEN).fold(Fp::from(1), |expr, j| {
                    expr * (Fp::from(chars[i]) - Fp::from(final_chars[j]))
                })
            };
            word_diffs_yellow[idx][i] = Value::known(Fp::from(yellow_diff).into());
        }
    }

    let circuit = WordleCircuit::<Fp> {
        poly_words,
        word_chars,
        word_diffs_green,
        word_diffs_yellow,
    };
    let empty_circuit = WordleCircuit::<Fp> {
        poly_words: [Value::unknown(); WORD_COUNT],
        word_chars: [[Value::unknown(); WORD_LEN]; WORD_COUNT],
        word_diffs_green: [[Value::unknown(); WORD_LEN]; WORD_COUNT],
        word_diffs_yellow: [[Value::unknown(); WORD_LEN]; WORD_COUNT],
    };


    let mut instance = Vec::new();

    // final word chars
    let mut final_chars_instance = vec![];
    for i in 0..WORD_LEN {
        final_chars_instance.push(Fp::from(final_chars[i]));
    }
    instance.push(final_chars_instance.clone());

    let mut diffs = vec![];
    for idx in 0..WORD_COUNT {
        diffs.push(compute_diff(&words[idx], &final_word));
    }

    // color green
    let mut green = vec![];
    for idx in 0..WORD_COUNT {
        for i in 0..WORD_LEN {
            green.push(diffs[idx][0][i]);
        }
    }
    instance.push(green.clone());

    // color yellow
    let mut yellow = vec![];
    for idx in 0..WORD_COUNT {
        for i in 0..WORD_LEN {
            yellow.push(diffs[idx][1][i]);
        }
    }
    instance.push(yellow.clone());

    let mut instance_slice = [
        &final_chars_instance.clone()[..],
        &green.clone()[..],
        &yellow.clone()[..],
    ];

    println!("Successfully generated witness");

    // // let params_fs = File::open("params.bin").unwrap();
    // // let params = Params::<EqAffine>::read(&mut BufReader::new(params_fs)).unwrap();
    let params = Params::<EqAffine>::read(&mut BufReader::new(&params_vec[..])).unwrap();

    let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
    let pk = keygen_pk(&params, vk.clone(), &empty_circuit).expect("keygen_pk should not fail");
    println!("Successfully generated proving key");

    let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
    // Create a proof
    create_proof(
        &params,
        &pk,
        &[circuit.clone(), circuit.clone()],
        &[&instance_slice, &instance_slice],
        OsRng,
        &mut transcript,
    )
    .expect("proof generation should not fail");
    let proof: Vec<u8> = transcript.finalize();
    // let proof = [1 as u8]; // delete
    JsValue::from_serde(&proof).unwrap()
}