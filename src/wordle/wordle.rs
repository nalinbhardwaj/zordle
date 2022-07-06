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

// to do: add checker if green = 0, then diff is not zero
// (1 - green) * is_zero(diff) = 0
// same for yellow

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
    char_yellow: [Column<Advice>; WORD_LEN],
    char_yellow_instance: [Column<Instance>; WORD_LEN],
    table: RangeTableConfig<F>,
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
        char_yellow: [Column<Advice>; WORD_LEN],
        char_yellow_instance: [Column<Instance>; WORD_LEN],
    ) -> Self {
        let q_lookup = meta.complex_selector();
        let table = RangeTableConfig::configure(meta);

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

        meta.create_gate("color check", |meta| {
            let q = meta.query_selector(q_lookup);
            
            let mut constraints = Vec::new();
            for idx in 0..WORD_LEN {
                let char = meta.query_advice(chars[idx], Rotation::cur());
                let final_char = meta.query_advice(final_word_chars[idx], Rotation::cur());
                let green = meta.query_advice(char_green[idx], Rotation::cur());
                constraints.push(q.clone() * (char.clone() - final_char.clone()) * green.clone());
            }

            for idx in 0..WORD_LEN {
                let char = meta.query_advice(chars[idx], Rotation::cur());
                let yellow = meta.query_advice(char_yellow[idx], Rotation::cur());

                let yellow_check = {
                    (0..WORD_LEN).fold(Expression::Constant(F::from(1)), |expr, i| {
                        let final_char = meta.query_advice(final_word_chars[i], Rotation::cur());
                        expr * (char.clone() - final_char)
                    })
                };
                constraints.push(q.clone() * yellow_check.clone() * yellow.clone());
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
            char_yellow,
            char_yellow_instance,
            table,
        }
    }

    pub fn assign_lookup(
        &self,
        mut layouter: impl Layouter<F>,
        poly_word: Value<Assigned<F>>,
        chars: [Value<Assigned<F>>; WORD_LEN],
        instance_offset: usize,
    ) -> Result<(), Error> {
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
        RangeCheckConfig::configure(meta,
            poly_word,
            chars,
            final_word_chars_advice,
            final_word_chars_instance,
            char_green,
            char_green_instance,
            char_yellow,
            char_yellow_instance
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
        let k = 9;

        let words = [String::from("abcde"), String::from("aaaaa")];
        
        let mut poly_words: [Value<Assigned<Fp>>; WORD_COUNT] = [Value::known(Fp::from(123).into()), Value::known(Fp::from(123).into())];
        let mut word_chars: [[Value<Assigned<Fp>>; WORD_LEN]; WORD_COUNT] = [[Value::known(Fp::from(123).into()); WORD_LEN]; WORD_COUNT];

        for idx in 0..WORD_COUNT {
            poly_words[idx] = Value::known(Fp::from(word_to_polyhash(&words[idx].clone())).into());
            let chars = word_to_chars(&words[idx].clone());
            for i in 0..WORD_LEN {
                word_chars[idx][i] = Value::known(Fp::from(chars[i]).into());
            }
        }

        // Successful cases
        let circuit = MyCircuit::<Fp> {
            poly_words,
            word_chars,
        };

        let mut instance = Vec::new();

        let final_word = String::from("edcba");
        let final_chars = word_to_chars(&final_word);
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
