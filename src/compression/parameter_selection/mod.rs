use crate::coding::rice_coding::RiceCoder;

const MAX_CONTEXT: usize = u8::MAX as usize;
const HALVE_AT: u32 = 1024;

/// This struct is used to estimate the optimal Rice parameter
/// value k from a given list of reasonable parameters for k.
pub struct KEstimator {
    reasonable_ks: Vec<u8>,

    // context_map[C][k] - the code length we would have had
    // if we had used parameter k to encode all values encountered
    // so far in the context C.
    context_map: Vec<Vec<u32>>,
    periodic_count_scaling: bool,
}

impl KEstimator {
    /// Creates a new KEstimator for the given set
    /// of k parameters.
    /// If `periodic_count_scaling` is set to `true`, try
    /// to exploit locality of reference within the image.
    ///
    /// # Panics
    /// Panics if the list of reasonable k values is empty.
    pub fn new(reasonable_ks: Vec<u8>, periodic_count_scaling: bool) -> KEstimator {
        if reasonable_ks.is_empty() {
            panic!("The list of k values is empty!");
        }

        let mut context_map = Vec::new();
        for _context in 0..=MAX_CONTEXT {
            let k = vec![0; reasonable_ks.len()];
            context_map.push(k);
        }

        return KEstimator {
            reasonable_ks,
            context_map,
            periodic_count_scaling,
        };
    }

    /// Updates the cumulative totals for this context
    /// to reflect that we have encoded a new value.
    /// If `periodic_count_scaling` is set to `true`, halve
    /// all code lengths in the context when the smallest one reaches
    /// a certain threshold.
    pub fn update(&mut self, context: u8, encoded: u32) {
        let ks_for_context = &mut self.context_map[context as usize];

        for (ki, &k) in self.reasonable_ks.iter().enumerate() {
            let code_length = RiceCoder::new(k).code_length(encoded);
            ks_for_context[ki] += code_length;
        }

        if self.periodic_count_scaling {
            let min_value = ks_for_context.iter().min().unwrap();
            if *min_value > HALVE_AT {
                ks_for_context.iter_mut().for_each(|x| *x /= 2);
            }
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
        let mut estimator = KEstimator::new(k_values.clone(), false);

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
        let mut estimator = KEstimator::new(k_values.clone(), false);

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

    #[test]
    #[should_panic]
    fn test_estimator_no_k_values() {
        KEstimator::new(vec![], false);
    }

    #[test]
    fn test_estimator_periodic_count_scaling() {
        let mut estimator = KEstimator::new(vec![0, 1, 2], true);
        let context = 43;

        estimator.update(context, 400);
        //  k:      0    1     2
        //  len:   401  202   103
        //  total: 401  202   103

        estimator.update(context, 531);
        //  k:    0    1     2
        //  len: 532  267   135
        //  total: 933 469  238

        estimator.update(context, 2000);
        //  k:      0     1     2
        //  len:   2001  1002 503
        //  total: 2934  1471 741

        estimator.update(context, 1733);
        //  k:      0     1     2
        //  len:   1734  868   436
        //  total: 4668 2339  1177 (before scaling)

        let ks = &estimator.context_map[context as usize];
        assert_eq!(ks[0], 2334);
        assert_eq!(ks[1], 1169);
        assert_eq!(ks[2], 588);
    }
}
