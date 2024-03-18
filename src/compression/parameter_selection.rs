use crate::coding::rice_coding::RiceCoder;

/// This struct is used to estimate the optimal Rice parameter
/// value k from a given list of reasonable parameters for k.
pub struct KEstimator {
    max_context: u32,
    k_values: &'static [u8],
    // context_map[C][k] - the code length we would have had
    // if we had used parameter k to encode all values encountered
    // so far in the context C.
    context_map: Vec<Vec<u32>>,
    halve_at: Option<u32>,
}

impl KEstimator {
    /// Creates a new KEstimator for the given set
    /// of k parameters.
    ///
    /// If `Some(value)` is passed, use periodic count scaling by halving all
    /// code lengths when the smallest one reaches `value`.
    ///
    /// # Panics
    /// Panics if the list of reasonable k values is empty.
    pub fn new(max_context: u32, k_values: &'static [u8], halve_at: Option<u32>) -> KEstimator {
        if k_values.is_empty() {
            panic!("The list of k values is empty!");
        }

        let mut context_map = Vec::new();
        for _context in 0..=max_context {
            let k = vec![0; k_values.len()];
            context_map.push(k);
        }

        return KEstimator {
            max_context,
            k_values,
            context_map,
            halve_at,
        };
    }

    /// Updates the cumulative totals for this context
    /// to reflect that we have encoded a new value.
    ///
    /// # Panics
    ///
    /// Panics if the context >= max_context.
    pub fn update(&mut self, context: u32, encoded: u32) {
        assert!(context < self.max_context);
        let ks_for_context = &mut self.context_map[context as usize];

        for (ki, &k) in self.k_values.iter().enumerate() {
            let code_length = RiceCoder::new(k).code_length(encoded);
            ks_for_context[ki] += code_length;
        }

        if let Some(halve_at) = self.halve_at {
            let min_value = ks_for_context.iter().min().unwrap();
            if *min_value > halve_at {
                ks_for_context.iter_mut().for_each(|x| *x /= 2);
            }
        }
    }

    /// Returns the best parameter value k for the current context.
    pub fn get_k(&self, context: u32) -> u8 {
        assert!(context < self.max_context);
        let ks_for_context = &self.context_map[context as usize];

        let mut smallest = u32::MAX;
        let mut best = 0;

        for (i, k) in ks_for_context.iter().enumerate() {
            if *k < smallest {
                best = i;
                smallest = *k;
            }
        }
        self.k_values[best]
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
        let k_values = &[0, 1, 2, 4, 8, 16];
        let mut estimator = KEstimator::new(300, k_values, None);

        let mut add_to_context: HashMap<u32, Vec<u32>> = HashMap::new();

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
        let k_values = &[0, 1, 2, 4, 5, 16];
        let mut estimator = KEstimator::new(400, k_values, None);

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
        KEstimator::new(100, &[], None);
    }

    #[test]
    fn test_estimator_periodic_count_scaling() {
        let mut estimator = KEstimator::new(120, &[0, 1, 2], Some(1024));
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
