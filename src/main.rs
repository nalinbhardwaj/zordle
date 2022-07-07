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
use std::io::{self, Write};

mod wordle;
use crate::wordle::wordle::{*, utils::*};

fn main() {
    println!("Hello, world!");
    let k = 15;

    let words = [String::from("audio"), String::from("hunky"), String::from("funky"), String::from("fluff")];
    
    let mut poly_words: [Value<Assigned<Fp>>; WORD_COUNT] = [Value::known(Fp::from(123).into()), Value::known(Fp::from(123).into()), Value::known(Fp::from(123).into()), Value::known(Fp::from(123).into())];
    let mut word_chars: [[Value<Assigned<Fp>>; WORD_LEN]; WORD_COUNT] = [[Value::known(Fp::from(123).into()); WORD_LEN]; WORD_COUNT];

    for idx in 0..WORD_COUNT {
        poly_words[idx] = Value::known(Fp::from(word_to_polyhash(&words[idx].clone())).into());
        let chars = word_to_chars(&words[idx].clone());
        for i in 0..WORD_LEN {
            word_chars[idx][i] = Value::known(Fp::from(chars[i]).into());
        }
    }

    let final_word = String::from("fluff");
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

    // println!("word_diffs_green {:?}", word_diffs_green);
    // println!("{:?}", word_diffs_yellow);

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

    let params: Params<EqAffine> = Params::new(k);

    let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
    let pk = keygen_pk(&params, vk, &empty_circuit).expect("keygen_pk should not fail");

    // let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
    // // Create a proof
    // create_proof(
    //     &params,
    //     &pk,
    //     &[circuit.clone(), circuit.clone()],
    //     &[&instance_slice, &instance_slice],
    //     OsRng,
    //     &mut transcript,
    // )
    // .expect("proof generation should not fail");
    // let proof: Vec<u8> = transcript.finalize();

    // std::fs::write("proof.bin", &proof[..])
    //     .expect("should succeed to write new proof");

    // io::stdout().write_all(&proof);

    // Check that a hardcoded proof is satisfied
    let proof = include_bytes!("proof.bin");
    let strategy = SingleVerifier::new(&params);
    let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
    assert!(verify_proof(
        &params,
        pk.get_vk(),
        strategy,
        &[&instance_slice, &instance_slice],
        &mut transcript,
    )
    .is_ok());
}