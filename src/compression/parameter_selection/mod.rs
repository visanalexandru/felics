use crate::coding::rice_coding::RiceCoder;

const MAX_CONTEXT: usize = u8::MAX as usize;

/// This struct is used to estimate the optimal Rice parameter
/// value k from a given list of reasonable parameters for k.
pub struct KEstimator {
    reasonable_ks: Vec<u8>,

    // context_map[C][k] - the code length we would have had
    // if we had used parameter k to encode all values encountered
    // so far in the context C.
    context_map: Vec<Vec<u32>>,
}

impl KEstimator {
    /// Creates a new KEstimator for the given set
    /// of k parameters.
    pub fn new(reasonable_ks: Vec<u8>) -> KEstimator {
        let mut context_map = Vec::new();

        for _context in 0..=MAX_CONTEXT {
            let k = vec![0; reasonable_ks.len()];
            context_map.push(k);
        }

        return KEstimator {
            reasonable_ks,
            context_map,
        };
    }

    /// Updates the cumulative totals for this context
    /// to reflect that we have encoded a new value.
    pub fn update(&mut self, context: u8, encoded: u32) {
        for (ki, &k) in self.reasonable_ks.iter().enumerate() {
            let code_length = RiceCoder::new(k).code_length(encoded);
            self.context_map[context as usize][ki] += code_length;
        }
    }

    /// Returns the best parameter value k for the current context.
    pub fn get_k(&self, context: u8) -> u8 {
        let ks_for_context = &self.context_map[context as usize];

        let (best_index, _min_length) = ks_for_context
            .iter()
            .enumerate()
            .min_by_key(|(_index, &total_length)| total_length)
            .unwrap();

        return self.reasonable_ks[best_index];
    }
}

#[cfg(test)]
mod test {
    use super::KEstimator;
    use crate::coding::rice_coding::RiceCoder;
    use std::collections::HashMap;

    /// Check the corectnes of the context_map after some updates.
    #[test]
    fn test_estimator_context_map() {
        let k_values = vec![0, 1, 2, 4, 8, 16];
        let mut estimator = KEstimator::new(k_values.clone());

        let mut add_to_context: HashMap<u8, Vec<u32>> = HashMap::new();

        add_to_context.insert(100, vec![4, 8, 13, 45, 85]);
        add_to_context.insert(80, vec![7, 800, 1000, 1273, 85]);
        add_to_context.insert(75, vec![7, 13, 1000, 200, 85]);
        add_to_context.insert(255, vec![1, 4, 142, 563, 1246, 2464]);
        add_to_context.insert(0, vec![0, 100, 3]);

        for (&context, values_in_context) in add_to_context.iter() {
            values_in_context
                .iter()
                .for_each(|&value| estimator.update(context, value));
        }

        for (&context, values_in_context) in add_to_context.iter() {
            for (i, &k) in k_values.iter().enumerate() {
                let coder = RiceCoder::new(k);
                let total_length: u32 = values_in_context
                    .iter()
                    .map(|&value| coder.code_length(value))
                    .sum();
                assert_eq!(total_length, estimator.context_map[context as usize][i]);
            }
        }
    }

    #[test]
    fn test_estimator_get_k() {
        let k_values = vec![0, 1, 2, 4, 5, 16];
        let mut estimator = KEstimator::new(k_values.clone());

        let context = 100;

        estimator.update(context, 10);
        estimator.update(context, 40);
        estimator.update(context, 5);

        assert_eq!(estimator.get_k(context), 4);

        let context = 255;

        estimator.update(context, 1000);
        estimator.update(context, 200);
        estimator.update(context, 1250);
        estimator.update(context, 300);
        assert_eq!(estimator.get_k(context), 16);
    }
}
