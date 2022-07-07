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

mod utils;
use utils::*;

mod is_zero;
use is_zero::*;


// This helper checks that the value witnessed in a given cell is within a a lookup dictionary table.

#[derive(Debug, Clone)]
/// A range-constrained value in the circuit produced by the RangeCheckConfig.
struct RangeConstrained<F: FieldExt>(AssignedCell<Assigned<F>, F>);

#[derive(Debug, Clone)]
struct RangeCheckConfig<F: FieldExt> {
    q_lookup: Selector,
    poly_word: Column<Advice>,
    chars: [Column<Advice>; WORD_LEN],
    final_word_chars: [Column<Advice>; WORD_LEN],
    final_word_chars_instance: [Column<Instance>; WORD_LEN],
    char_green: [Column<Advice>; WORD_LEN],
    char_green_instance: [Column<Instance>; WORD_LEN],
    diffs_green: [Column<Advice>; WORD_LEN],
    char_yellow: [Column<Advice>; WORD_LEN],
    char_yellow_instance: [Column<Instance>; WORD_LEN],
    diffs_yellow: [Column<Advice>; WORD_LEN],
    table: RangeTableConfig<F>,
    diffs_green_is_zero: [IsZeroConfig<F>; WORD_LEN],
    diffs_yellow_is_zero: [IsZeroConfig<F>; WORD_LEN],
}

impl<F: FieldExt>
    RangeCheckConfig<F>
{
    pub fn configure(meta: &mut ConstraintSystem<F>,
        poly_word: Column<Advice>,
        chars: [Column<Advice>; WORD_LEN],
        final_word_chars: [Column<Advice>; WORD_LEN],
        final_word_chars_instance: [Column<Instance>; WORD_LEN],
        char_green: [Column<Advice>; WORD_LEN],
        char_green_instance: [Column<Instance>; WORD_LEN],
        diffs_green: [Column<Advice>; WORD_LEN],
        char_yellow: [Column<Advice>; WORD_LEN],
        char_yellow_instance: [Column<Instance>; WORD_LEN],
        diffs_yellow: [Column<Advice>; WORD_LEN],
    ) -> Self {
        let q_lookup = meta.complex_selector();
        let table = RangeTableConfig::configure(meta);

        let mut diffs_green_is_zero = vec![];
        let mut diffs_yellow_is_zero = vec![];
        for i in 0..WORD_LEN {
            let green_is_zero_advice_column = meta.advice_column();
            diffs_green_is_zero.push(IsZeroChip::configure(
                meta,
                |meta| meta.query_selector(q_lookup),
                |meta| meta.query_advice(diffs_green[i], Rotation::cur()),
                green_is_zero_advice_column,
            ));

            let yellow_is_zero_advice_column = meta.advice_column();
            diffs_yellow_is_zero.push(IsZeroChip::configure(
                meta,
                |meta| meta.query_selector(q_lookup),
                |meta| meta.query_advice(diffs_yellow[i], Rotation::cur()),
                yellow_is_zero_advice_column,
            ));
        }

        for i in 0..WORD_LEN {
            meta.enable_equality(chars[i]);
            meta.enable_equality(final_word_chars[i]);
            meta.enable_equality(final_word_chars_instance[i]);
            meta.enable_equality(char_green[i]);
            meta.enable_equality(char_green_instance[i]);
            meta.enable_equality(char_yellow[i]);
            meta.enable_equality(char_yellow_instance[i]);
        }

        meta.lookup(|meta| {
            let q_lookup = meta.query_selector(q_lookup);
            let poly_word = meta.query_advice(poly_word, Rotation::cur());

            vec![(q_lookup * poly_word, table.value)] // check if q_lookup * value is in the table.
        });

        meta.create_gate("poly hashing check", |meta| {
            let q = meta.query_selector(q_lookup);
            let poly_word = meta.query_advice(poly_word, Rotation::cur());

            let hash_check = {
                (0..WORD_LEN).fold(Expression::Constant(F::from(0)), |expr, i| {
                    let char = meta.query_advice(chars[i], Rotation::cur());
                    expr * Expression::Constant(F::from(BASE)) + char
                })
            };

            [q * (hash_check - poly_word)]
        });

        meta.create_gate("color check",|meta | {
            let q = meta.query_selector(q_lookup);
            
            let mut constraints = Vec::new();
            for idx in 0..WORD_LEN {
                let char = meta.query_advice(chars[idx], Rotation::cur());
                let final_char = meta.query_advice(final_word_chars[idx], Rotation::cur());
                let green = meta.query_advice(char_green[idx], Rotation::cur());
                let diff_green = meta.query_advice(diffs_green[idx], Rotation::cur());
                constraints.push(q.clone() * (char.clone() - final_char.clone()) * green.clone());
                constraints.push(q.clone() * diffs_green_is_zero[idx].expr() * (Expression::Constant(F::one()) - green.clone()));
                constraints.push(q.clone() * ((char.clone() - final_char.clone()) - diff_green.clone()));
            }

            for idx in 0..WORD_LEN {
                let char = meta.query_advice(chars[idx], Rotation::cur());
                let yellow = meta.query_advice(char_yellow[idx], Rotation::cur());
                let diff_yellow = meta.query_advice(diffs_yellow[idx], Rotation::cur());

                let yellow_check = {
                    (0..WORD_LEN).fold(Expression::Constant(F::one()), |expr, i| {
                        let final_char = meta.query_advice(final_word_chars[i], Rotation::cur());
                        expr * (char.clone() - final_char)
                    })
                };
                constraints.push(q.clone() * yellow_check.clone() * yellow.clone());
                constraints.push(q.clone() * diffs_yellow_is_zero[idx].expr() * (Expression::Constant(F::one()) - yellow.clone()));
                constraints.push(q.clone() * (yellow_check.clone() - diff_yellow.clone()));
            }

            constraints
        });

        meta.create_gate("character range check", |meta| {
            let q = meta.query_selector(q_lookup);
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

        Self {
            q_lookup,
            poly_word,
            chars,
            final_word_chars,
            final_word_chars_instance,
            char_green,
            char_green_instance,
            diffs_green,
            char_yellow,
            char_yellow_instance,
            table,
            diffs_yellow,
            diffs_green_is_zero: diffs_green_is_zero.try_into().unwrap(),
            diffs_yellow_is_zero: diffs_yellow_is_zero.try_into().unwrap(),
        }
    }

    pub fn assign_lookup(
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
            || "Assign value for lookup dictionary check",
            |mut region| {
                let offset = 0;

                // Enable q_lookup
                self.q_lookup.enable(&mut region, offset)?;

                // Assign value
                region
                    .assign_advice(|| "poly word", self.poly_word, offset, || poly_word)
                    .map(RangeConstrained)?;
                
                for i in 0..WORD_LEN {
                    region.assign_advice(|| "characters", self.chars[i], offset, || chars[i])?;
                    region.assign_advice(|| "characters", self.diffs_green[i], offset, || diffs_green[i])?;
                    region.assign_advice(|| "characters", self.diffs_yellow[i], offset, || diffs_yellow[i])?;
                    diffs_green_is_zero_chips[i].assign(&mut region, 0, diffs_green[i])?;
                    diffs_yellow_is_zero_chips[i].assign(&mut region, 0, diffs_yellow[i])?;

                    region.assign_advice_from_instance(|| "final word characters",
                    self.final_word_chars_instance[i], 0, self.final_word_chars[i], offset)?;
                    region.assign_advice_from_instance(|| "color green chars",
                    self.char_green_instance[i], instance_offset, self.char_green[i], offset)?;
                    region.assign_advice_from_instance(|| "color yellow chars",
                    self.char_yellow_instance[i], instance_offset, self.char_yellow[i], offset)?;
                }

                Ok(())
            },
        )
    }
}


#[derive(Default)]
struct MyCircuit<F: FieldExt> {
    poly_words: [Value<Assigned<F>>; WORD_COUNT],
    word_chars: [[Value<Assigned<F>>; WORD_LEN]; WORD_COUNT],
    word_diffs_green: [[Value<F>; WORD_LEN]; WORD_COUNT],
    word_diffs_yellow: [[Value<F>; WORD_LEN]; WORD_COUNT],
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F>
{
    type Config = RangeCheckConfig<F>;
    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let poly_word = meta.advice_column();
        let chars = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column()            
        ];
        let final_word_chars_instance = [
            meta.instance_column(),
            meta.instance_column(),
            meta.instance_column(),
            meta.instance_column(),
            meta.instance_column(),            
        ];
        let final_word_chars_advice = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column()            
        ];
        let char_yellow = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column()            
        ];
        let char_green = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column()            
        ];
        let char_green_instance = [
            meta.instance_column(),
            meta.instance_column(),
            meta.instance_column(),
            meta.instance_column(),
            meta.instance_column()            
        ];
        let char_yellow_instance = [
            meta.instance_column(),
            meta.instance_column(),
            meta.instance_column(),
            meta.instance_column(),
            meta.instance_column()            
        ];
        let diffs_green = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column()            
        ];
        let diffs_yellow = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column()            
        ];
        RangeCheckConfig::configure(meta,
            poly_word,
            chars,
            final_word_chars_advice,
            final_word_chars_instance,
            char_green,
            char_green_instance,
            diffs_green,
            char_yellow,
            char_yellow_instance,
            diffs_yellow
        )
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        config.table.load(&mut layouter)?;

        for idx in 0..WORD_COUNT {
            config.assign_lookup(
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
    fn test_range_check_2() {
        let k = 16;

        let words = [String::from("audio"), String::from("hunky"), String::from("fluff")];
        
        let mut poly_words: [Value<Assigned<Fp>>; WORD_COUNT] = [Value::known(Fp::from(123).into()), Value::known(Fp::from(123).into()), Value::known(Fp::from(123).into())];
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

        println!("word_diffs_green {:?}", word_diffs_green);
        println!("{:?}", word_diffs_yellow);

        // Successful cases
        let circuit = MyCircuit::<Fp> {
            poly_words,
            word_chars,
            word_diffs_green,
            word_diffs_yellow,
        };

        let mut instance = Vec::new();

        // final word chars
        for i in 0..WORD_LEN {
            instance.push(vec![Fp::from(final_chars[i])]);
        }
        let mut diffs = vec![];
        for idx in 0..WORD_COUNT {
            diffs.push(compute_diff(&words[idx], &final_word));
        }

        // color green
        for i in 0..WORD_LEN {
            let mut row = vec![];
            for idx in 0..WORD_COUNT {
                row.push(diffs[idx][0][i]);
            }
            instance.push(row);
        }

        // color yellow
        for i in 0..WORD_LEN {
            let mut row = vec![];
            for idx in 0..WORD_COUNT {
                row.push(diffs[idx][1][i]);
            }
            instance.push(row);
        }

        println!("{:?}", instance);

        let prover = MockProver::run(k, &circuit, instance).unwrap();
        prover.assert_satisfied();

    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn print_range_check_2() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("range-check-2-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root
            .titled("Range Check 2 Layout", ("sans-serif", 60))
            .unwrap();

        let circuit = MyCircuit::<Fp, 8, 256> {
            value: Value::unknown(),
            lookup_value: Value::unknown(),
        };
        halo2_proofs::dev::CircuitLayout::default()
            .render(9, &circuit, &root)
            .unwrap();
    }
}
