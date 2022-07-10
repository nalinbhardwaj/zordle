use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Layouter, Value},
    plonk::{ConstraintSystem, Error, TableColumn},
};

use serde::{Deserialize, Serialize};

use std::error::Error as StdError;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::utils::*;
use super::dict::*;

#[derive(Serialize, Deserialize)]
struct Dict {
    words: Vec<String>,
}

/// A lookup table of values from dictionary.
#[derive(Debug, Clone)]
pub(super) struct DictTableConfig<F: FieldExt> {
    pub(super) value: TableColumn,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> DictTableConfig<F> {
    pub(super) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let value = meta.lookup_table_column();

        Self {
            value,
            _marker: PhantomData,
        }
    }

    pub(super) fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        let mut words = get_dict();
        // println!("words {:?}", words);
        words.push(0);

        layouter.assign_table(
            || "load dictionary-check table",
            |mut table| {
                let mut offset = 0;
                for word in words.iter() {
                    table.assign_cell(
                        || "num_bits",
                        self.value,
                        offset,
                        || Value::known(F::from(word.clone() as u64)),
                    )?;
                    offset += 1;
                }

                Ok(())
            },
        )
    }
}
