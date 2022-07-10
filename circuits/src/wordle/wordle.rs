use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter, Value},
    plonk::{Advice, Assigned, Column, ConstraintSystem, Constraints, Error, Expression, Selector, Instance},
    poly::Rotation,
};
use halo2_proofs::{
    circuit::floor_planner::V1,
    dev::{FailureLocation, MockProver, VerifyFailure},
    pasta::Fp,
    plonk::{Any, Circuit},
};

mod table;
use table::*;

pub mod utils;
use utils::*;

pub mod dict;

mod is_zero;
use is_zero::*;


// This helper checks that the value witnessed in a given cell is within a a lookup dictionary table.

#[derive(Debug, Clone)]
/// A range-constrained value in the circuit produced by the RangeCheckConfig.
struct RangeConstrained<F: FieldExt>(AssignedCell<Assigned<F>, F>);

#[derive(Debug, Clone)]
pub struct WordCheckConfig<F: FieldExt> {
    q_input: Selector,
    q_diff_g: Selector,
    q_diff_y: Selector,
    q_diff_green_is_zero: Selector,
    q_diff_yellow_is_zero: Selector,
    q_color_is_zero: Selector,
    q_color: Selector,
    poly_word: Column<Advice>,
    chars: [Column<Advice>; WORD_LEN],
    color_is_zero_advice_column: [Column<Advice>; WORD_LEN],
    final_word_chars_instance: Column<Instance>,
    char_green_instance: Column<Instance>,
    char_yellow_instance: Column<Instance>,
    table: DictTableConfig<F>,
    diffs_green_is_zero: [IsZeroConfig<F>; WORD_LEN],
    diffs_yellow_is_zero: [IsZeroConfig<F>; WORD_LEN],
}

// instance rows:   poly    chr0    chr1    chr2    chr3    chr4    q_dict  q_poly  q_color  q_range_check
// word                                                                                         1
// final
// diffs_g
// diffs_y
// diffs_g_0
// diffs_y_0
// green
// yellow

impl<F: FieldExt>
    WordCheckConfig<F>
{
    pub fn configure(meta: &mut ConstraintSystem<F>,
        q_input: Selector,
        q_diff_g: Selector,
        q_diff_y: Selector,
        q_diff_green_is_zero: Selector,
        q_diff_yellow_is_zero: Selector,
        q_color_is_zero: Selector,
        q_color: Selector,
        poly_word: Column<Advice>,
        chars: [Column<Advice>; WORD_LEN],
        color_is_zero_advice_column: [Column<Advice>; WORD_LEN],
        final_word_chars_instance: Column<Instance>,
        char_green_instance: Column<Instance>,
        char_yellow_instance: Column<Instance>,
    ) -> Self {
        let table = DictTableConfig::configure(meta);

        let mut diffs_green_is_zero = vec![];
        let mut diffs_yellow_is_zero = vec![];
        for i in 0..WORD_LEN {
            diffs_green_is_zero.push(IsZeroChip::configure(
                meta,
                |meta| meta.query_selector(q_diff_green_is_zero),
                |meta| meta.query_advice(chars[i], Rotation(-2)),
                color_is_zero_advice_column[i],
            ));

            diffs_yellow_is_zero.push(IsZeroChip::configure(
                meta,
                |meta| meta.query_selector(q_diff_yellow_is_zero),
                |meta| meta.query_advice(chars[i], Rotation(-2)),
                color_is_zero_advice_column[i],
            ));
        }

        for i in 0..WORD_LEN {
            meta.enable_equality(chars[i]);
        }
        meta.enable_equality(final_word_chars_instance);
        meta.enable_equality(char_green_instance);
        meta.enable_equality(char_yellow_instance);

        meta.lookup(|meta| {
            let q_lookup = meta.query_selector(q_input);
            let poly_word = meta.query_advice(poly_word, Rotation::cur());

            vec![(q_lookup * poly_word, table.value)] // check if q_lookup * value is in the table.
        });


        meta.create_gate("character range check", |meta| {
            let q = meta.query_selector(q_input);
            let mut constraints = vec![];
            for idx in 0..WORD_LEN {
                let value = meta.query_advice(chars[idx], Rotation::cur());

                let range_check = |range: usize, value: Expression<F>| {
                    assert!(range > 0);
                    (1..range).fold(value.clone(), |expr, i| {
                        expr * (Expression::Constant(F::from(i as u64)) - value.clone())
                    })
                };

                constraints.push(q.clone() * range_check(28, value.clone()));
            }

            constraints
        });

        meta.create_gate("poly hashing check", |meta| {
            let q = meta.query_selector(q_input);
            let poly_word = meta.query_advice(poly_word, Rotation::cur());

            let hash_check = {
                (0..WORD_LEN).fold(Expression::Constant(F::from(0)), |expr, i| {
                    let char = meta.query_advice(chars[i], Rotation::cur());
                    expr * Expression::Constant(F::from(BASE)) + char
                })
            };

            [q * (hash_check - poly_word)]
        });

        meta.create_gate("diff_g checker", |meta| {
            let q = meta.query_selector(q_diff_g);
            let mut constraints = vec![];
            for i in 0..WORD_LEN {
                let char = meta.query_advice(chars[i], Rotation(-2));
                let final_char = meta.query_advice(chars[i], Rotation(-1));
                let diff_g = meta.query_advice(chars[i], Rotation::cur());
                constraints.push(q.clone() * ((char - final_char) - diff_g));
                // constraints.push(q.clone() * Expression::Constant(F::zero()));
            }

            constraints
        });

        meta.create_gate("diff_y checker", |meta| {
            let q = meta.query_selector(q_diff_y);
            let mut constraints = vec![];
            for i in 0..WORD_LEN {
                let char = meta.query_advice(chars[i], Rotation(-3));
                let diff_y = meta.query_advice(chars[i], Rotation::cur());

                let yellow_check = {
                    (0..WORD_LEN).fold(Expression::Constant(F::one()), |expr, i| {
                        let final_char = meta.query_advice(chars[i], Rotation(-2));
                        expr * (char.clone() - final_char)
                    })
                };
                constraints.push(q.clone() * (yellow_check - diff_y));
            }

            constraints
        });

        meta.create_gate("diff_color_is_zero checker", |meta| {
            let q_green = meta.query_selector(q_diff_green_is_zero);
            let q_yellow = meta.query_selector(q_diff_yellow_is_zero);
            let q_color_is_zero = meta.query_selector(q_color_is_zero);
            let mut constraints = vec![];

            for i in 0..WORD_LEN {
                let diff_color_is_zero = meta.query_advice(chars[i], Rotation::cur());
                constraints.push(q_color_is_zero.clone() * (diff_color_is_zero - (q_green.clone() * diffs_green_is_zero[i].expr() + q_yellow.clone() * diffs_yellow_is_zero[i].expr())));
                // constraints.push(q_color_is_zero.clone() * diffs_green_is_zero[i].expr() * q_yellow.clone());
            }

            constraints
        });

        meta.create_gate("color check", |meta| {
            let q = meta.query_selector(q_color);
            
            let mut constraints = vec![];
            for i in 0..WORD_LEN {
                let diff_color = meta.query_advice(chars[i], Rotation(-4));
                let diff_color_is_zero = meta.query_advice(chars[i], Rotation(-2));
                let color = meta.query_advice(chars[i], Rotation::cur());
                constraints.push(q.clone() * diff_color * color.clone());
                constraints.push(q.clone() * diff_color_is_zero * (Expression::Constant(F::one()) - color.clone()));
            }

            constraints
        });

        Self {
            q_input,
            q_diff_g,
            q_diff_y,
            q_diff_green_is_zero,
            q_diff_yellow_is_zero,
            q_color_is_zero,
            q_color,
            poly_word,
            chars,
            color_is_zero_advice_column,
            final_word_chars_instance,
            char_green_instance,
            char_yellow_instance,
            table,
            diffs_green_is_zero: diffs_green_is_zero.try_into().unwrap(),
            diffs_yellow_is_zero: diffs_yellow_is_zero.try_into().unwrap(),
        }
    }

    pub fn assign_word(
        &self,
        mut layouter: impl Layouter<F>,
        poly_word: Value<Assigned<F>>,
        chars: [Value<Assigned<F>>; WORD_LEN],
        diffs_green: [Value<F>; WORD_LEN],
        diffs_yellow: [Value<F>; WORD_LEN],
        instance_offset: usize,
    ) -> Result<(), Error> {
        let mut diffs_green_is_zero_chips = vec![];
        let mut diffs_yellow_is_zero_chips = vec![];
        for i in 0..WORD_LEN {
            diffs_green_is_zero_chips.push(IsZeroChip::construct(self.diffs_green_is_zero[i].clone()));
            diffs_yellow_is_zero_chips.push(IsZeroChip::construct(self.diffs_yellow_is_zero[i].clone()));
        }

        layouter.assign_region(
            || "one word checks",
            |mut region| {
                self.q_input.enable(&mut region, 0)?;
                self.q_diff_g.enable(&mut region, 2)?;
                self.q_diff_y.enable(&mut region, 3)?;
                self.q_diff_green_is_zero.enable(&mut region, 4)?;
                self.q_color_is_zero.enable(&mut region, 4)?;
                self.q_diff_yellow_is_zero.enable(&mut region, 5)?;
                self.q_color_is_zero.enable(&mut region, 5)?;
                self.q_color.enable(&mut region, 6)?;
                self.q_color.enable(&mut region, 7)?;

                // Assign value
                region
                    .assign_advice(|| "poly word", self.poly_word, 0, || poly_word)
                    .map(RangeConstrained)?;
                
                for i in 0..WORD_LEN {
                    region.assign_advice(|| "input word characters", self.chars[i], 0, || chars[i])?;
                    region.assign_advice_from_instance(|| "final word characters",
                    self.final_word_chars_instance, i, self.chars[i], 1)?;
                    region.assign_advice(|| "diff_g", self.chars[i], 2, || diffs_green[i])?;
                    region.assign_advice(|| "diff_y", self.chars[i], 3, || diffs_yellow[i])?;

                    diffs_green_is_zero_chips[i].assign(&mut region, 4, diffs_green[i])?;
                    diffs_yellow_is_zero_chips[i].assign(&mut region, 5, diffs_yellow[i])?;

                    let diff_g_is_zero = diffs_green[i].and_then(|v| {
                        if v == F::zero() {
                            Value::known(F::one())
                        } else {
                            Value::known(F::zero())
                        }
                    });
                    // println!("i: {:?} diff_g_is_zero: {:?}", i, diff_g_is_zero);
                    region.assign_advice(|| "diff_g_is_zero", self.chars[i], 4, || diff_g_is_zero)?;
                    let diff_y_is_zero = diffs_yellow[i].and_then(|v| {
                        if v == F::zero() {
                            Value::known(F::one())
                        } else {
                            Value::known(F::zero())
                        }
                    });
                    region.assign_advice(|| "diff_y_is_zero", self.chars[i], 5, || diff_y_is_zero)?;

                    region.assign_advice_from_instance(|| "color green",
                    self.char_green_instance, instance_offset * WORD_LEN + i, self.chars[i], 6)?;
                    region.assign_advice_from_instance(|| "color yellow",
                    self.char_yellow_instance, instance_offset * WORD_LEN + i, self.chars[i], 7)?;
                }

                Ok(())
            },
        )
    }
}


#[derive(Default, Clone)]
pub struct WordleCircuit<F: FieldExt> {
    pub poly_words: [Value<Assigned<F>>; WORD_COUNT],
    pub word_chars: [[Value<Assigned<F>>; WORD_LEN]; WORD_COUNT],
    pub word_diffs_green: [[Value<F>; WORD_LEN]; WORD_COUNT],
    pub word_diffs_yellow: [[Value<F>; WORD_LEN]; WORD_COUNT],
}

impl<F: FieldExt> Circuit<F> for WordleCircuit<F>
{
    type Config = WordCheckConfig<F>;
    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let q_input = meta.complex_selector();
        let q_diff_g = meta.selector();
        let q_diff_y = meta.selector();
        let q_diff_green_is_zero = meta.complex_selector();
        let q_diff_yellow_is_zero = meta.complex_selector();
        let q_color_is_zero = meta.selector();
        let q_color = meta.selector();

        let poly_word = meta.advice_column();
        let chars = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column()            
        ];
        let color_is_zero_advice_column = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column()
        ];
        let final_word_chars_instance = meta.instance_column();
        let char_green_instance = meta.instance_column();
        let char_yellow_instance = meta.instance_column();

        WordCheckConfig::configure(meta,
            q_input,
            q_diff_g,
            q_diff_y,
            q_diff_green_is_zero,
            q_diff_yellow_is_zero,
            q_color_is_zero,
            q_color,
            poly_word,
            chars,
            color_is_zero_advice_column,
            final_word_chars_instance,
            char_green_instance,
            char_yellow_instance,
        )
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        config.table.load(&mut layouter)?;

        for idx in 0..WORD_COUNT {
            // println!("idx {:?} diffs_green: {:?}", idx, self.word_diffs_green[idx]);
            config.assign_word(
                layouter.namespace(|| format!("word {}", idx)),
                self.poly_words[idx],
                self.word_chars[idx],
                self.word_diffs_green[idx],
                self.word_diffs_yellow[idx],
                idx,
            )?;
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wordle_1() {
        let k = 14;

        let words = [String::from("audio"), String::from("hunky"), String::from("funky"), String::from("fluff"), String::from("fluff"), String::from("fluff")];
        
        let mut poly_words: [Value<Assigned<Fp>>; WORD_COUNT] = [Value::known(Fp::from(123).into()); WORD_COUNT];
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

        // Successful cases
        let circuit = WordleCircuit::<Fp> {
            poly_words,
            word_chars,
            word_diffs_green,
            word_diffs_yellow,
        };

        let mut instance = Vec::new();

        // final word chars
        let mut final_chars_instance = vec![];
        for i in 0..WORD_LEN {
            final_chars_instance.push(Fp::from(final_chars[i]));
        }
        instance.push(final_chars_instance);

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
        instance.push(green);

        // color yellow
        let mut yellow = vec![];
        for idx in 0..WORD_COUNT {
            for i in 0..WORD_LEN {
                yellow.push(diffs[idx][1][i]);
            }
        }
        instance.push(yellow);

        // println!("instance {:?}", instance);

        let prover = MockProver::run(k, &circuit, instance).unwrap();
        prover.assert_satisfied();

    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn print_wordle() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("wordle-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root
            .titled("Wordle Layout", ("sans-serif", 60))
            .unwrap();

        let words = [String::from("audio"), String::from("hunky"), String::from("funky"), String::from("fluff"), String::from("fluff"), String::from("fluff")];
    
        let mut poly_words: [Value<Assigned<Fp>>; WORD_COUNT] = [Value::known(Fp::from(123).into()); WORD_COUNT];
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

        // Successful cases
        let circuit = WordleCircuit::<Fp> {
            poly_words,
            word_chars,
            word_diffs_green,
            word_diffs_yellow,
        };
        
        halo2_proofs::dev::CircuitLayout::default()
            .render(9, &circuit, &root)
            .unwrap();
    }
}
