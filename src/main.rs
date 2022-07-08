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
use std::io::{self, Write, Read, BufReader};

mod wordle;
use crate::wordle::wordle::{*, utils::*};

const K: u32 = 14;

fn interpret_diff(diff: Vec<Vec<Fp>>) {
    let mut diff_str = String::new();
    for i in 0..5 {
        if diff[0][i] == Fp::one() {
            diff_str.push('🟩');
        } else if diff[1][i] == Fp::one() {
            diff_str.push('🟨');
        } else {
            diff_str.push('🟥');
        }
    }
    println!("{}", diff_str);
}

fn verify_play(final_word: String) {

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

    // read json file to array
    let mut file = File::open("diffs_json.bin").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let diff_json: Vec<Vec<Vec<u64>>> = serde_json::from_str(&contents).unwrap();
    let mut diffs = vec![];

    for idx in 0..WORD_COUNT {
        let mut diff_instance = vec![];
        for i in 0..2 {
            let mut col_row = vec![];
            for j in 0..WORD_LEN {
                col_row.push(Fp::from(diff_json[idx][i][j]));
            }
            diff_instance.push(col_row);
        }
        diffs.push(diff_instance.clone());
    }

    println!("Verifying proof for final word {}", final_word);
    println!("Share Sheet:");
    for i in 0..WORD_COUNT {
        interpret_diff(diffs[i].clone());
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

    let params_fs = File::open("params.bin").unwrap();
    let params = Params::<EqAffine>::read(&mut BufReader::new(params_fs)).unwrap();

    let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");

    // Check that a hardcoded proof is satisfied
    let proof = include_bytes!("../proof.bin");
    let strategy = SingleVerifier::new(&params);
    let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
    let result = verify_proof(
        &params,
        &vk,
        strategy,
        &[&instance_slice, &instance_slice],
        &mut transcript,
    );
    if result.is_ok() {
        println!("Proof OK!");
    } else {
        println!("Proof not OK!");
    }
}

fn prove_play(words: [String; WORD_COUNT], final_word: String) {    
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

    let mut diffs_u64 = vec![];
    for idx in 0..WORD_COUNT {
        diffs_u64.push(compute_diff_u64(&words[idx], &final_word));
    }

    let diffs_json_str = serde_json::to_string(&diffs_u64).unwrap();
    let mut diffs_json_file = File::create("diffs_json.bin").unwrap();
    diffs_json_file.write_all(diffs_json_str.as_bytes()).unwrap();

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

    let params_fs = File::open("params.bin").unwrap();
    let params = Params::<EqAffine>::read(&mut BufReader::new(params_fs)).unwrap();

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

    std::fs::write("proof.bin", &proof[..])
        .expect("should succeed to write new proof");

    println!("Successfully wrote proof to proof.bin");

    println!("Verifying proof for final word {}", final_word);
    println!("Share Sheet:");
    for i in 0..WORD_COUNT {
        interpret_diff(diffs[i].clone());
    }

    // Check that a hardcoded proof is satisfied
    // let proof = include_bytes!("proof.bin");
    let strategy = SingleVerifier::new(&params);
    let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
    let result = verify_proof(
        &params,
        &vk,
        strategy,
        &[&instance_slice, &instance_slice],
        &mut transcript,
    );
    if result.is_ok() {
        println!("Proof OK!");
    } else {
        println!("Proof not OK!");
    }
}

fn write_params() {
    let mut params_file = File::create("params.bin").unwrap();
    let params: Params<EqAffine> = Params::new(K);
    params.write(&mut params_file).unwrap();
}

fn play(final_word: String) {

    let mut running = true;
    let mut counter = 0;
    let mut words = vec![];
    while running && counter < WORD_COUNT {
        println!("Enter a word:");
        let mut word = String::new();
        io::stdin().read_line(&mut word).unwrap();
        word = word.trim().to_string();
        assert!(word.len() == 5);
        words.push(word.clone());

        let diff = compute_diff(&words[counter], &final_word);
        interpret_diff(diff);

        if word == final_word {
            running = false;
        }
        counter += 1;
    }

    while counter < 6 {
        words.push(final_word.clone());
        counter += 1;
    }

    if !running {
        println!("You win! Generating ZK proof...");
        prove_play(words.try_into().unwrap(), final_word);
    } else {
        println!("You lose!");
    }
    
}

fn main() {
    let final_word = "fluff".to_string();
    println!("Welcome to zk wordle!");
    println!("Enter play to play the game, verify to check a proof, or write to generate a new params file");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input = input.trim().to_string();
    if input == "play" {
        play(final_word);
    } else if input == "verify" {
        verify_play(final_word);
    } else if input == "write" {
        write_params();
    } else {
        println!("Invalid input");
    }
}