use super::traits::Intensity;
use crate::coding::rice_coding::RiceCoder;
use std::marker::PhantomData;

/// This struct is used to estimate the optimal Rice parameter
/// value k to encode intensities of type `T`.
pub struct KEstimator<T>
where
    T: Intensity,
{
    // context_map[C][k] - the code length we would have had
    // if we had used parameter k to encode all values encountered
    // so far in the context C.
    context_map: Vec<Vec<u32>>,
    periodic_count_scaling: bool,
    phantom: PhantomData<T>,
}

impl<T> KEstimator<T>
where
    T: Intensity,
{
    /// Creates a new KEstimator.
    /// If `periodic_count_scaling` is set to `true`, try
    /// to exploit locality of reference within the image.
    pub fn new(periodic_count_scaling: bool) -> KEstimator<T> {
        assert!(!T::K_VALUES.is_empty());

        let mut context_map = Vec::new();
        for _context in 0..=T::MAX_CONTEXT {
            let k = vec![0; T::K_VALUES.len()];
            context_map.push(k);
        }

        return KEstimator {
            context_map,
            periodic_count_scaling,
            phantom: PhantomData,
        };
    }

    /// Updates the cumulative totals for this context
    /// to reflect that we have encoded a new value.
    /// If `periodic_count_scaling` is set to `true`, halve
    /// all code lengths in the context when the smallest one reaches
    /// a certain threshold.
    pub fn update(&mut self, context: T, encoded: T) {
        let context: usize = context.into();
        let ks_for_context = &mut self.context_map[context];

        for (ki, &k) in T::K_VALUES.iter().enumerate() {
            let code_length = RiceCoder::new(k).code_length(encoded.into());
            ks_for_context[ki] += code_length;
        }

        if self.periodic_count_scaling {
            let min_value = ks_for_context.iter().min().unwrap();
            if *min_value > T::COUNT_SCALING_THRESHOLD {
                ks_for_context.iter_mut().for_each(|x| *x /= 2);
            }
        }
    }

    /// Returns the best parameter value k for the current context.
    pub fn get_k(&self, context: T) -> u8 {
        let context: usize = context.into();
        let ks_for_context = &self.context_map[context];

        let mut smallest = u32::MAX;
        let mut best = 0;

        for (i, k) in ks_for_context.iter().enumerate() {
            if *k < smallest {
                best = i;
                smallest = *k;
            }
        }
        return T::K_VALUES[best];
    }
}

#[cfg(test)]
mod test {
    use super::{Intensity, KEstimator, RiceCoder};
    use std::collections::HashMap;
    /// Check the corectnes of the context_map after some updates.
    #[test]
    fn test_estimator_context_map() {
        let mut estimator: KEstimator<u8> = KEstimator::new(false);

        let mut add_to_context: HashMap<u8, Vec<u8>> = HashMap::new();
        add_to_context.insert(100, vec![4, 8, 13, 45, 85]);
        add_to_context.insert(80, vec![7, 30, 221, 43, 85]);
        add_to_context.insert(75, vec![7, 13, 187, 78, 85]);
        add_to_context.insert(255, vec![1, 4, 255, 100, 99, 71]);
        add_to_context.insert(0, vec![0, 100, 3]);

        for (&context, values_in_context) in add_to_context.iter() {
            values_in_context
                .iter()
                .for_each(|&value| estimator.update(context, value));
        }

        for (&context, values_in_context) in add_to_context.iter() {
            for (i, &k) in u8::K_VALUES.iter().enumerate() {
                let coder = RiceCoder::new(k);
                let total_length: u32 = values_in_context
                    .iter()
                    .map(|&value| coder.code_length(value.into()))
                    .sum();
                assert_eq!(total_length, estimator.context_map[context as usize][i]);
            }
        }
    }

    #[test]
    fn test_estimator_get_k() {
        let mut estimator: KEstimator<u8> = KEstimator::new(false);

        let context = 100;

        estimator.update(context, 10);
        estimator.update(context, 40);
        estimator.update(context, 5);

        assert_eq!(estimator.get_k(context), 4);

        let context = 255;

        estimator.update(context, 23);
        estimator.update(context, 255);
        estimator.update(context, 255);
        estimator.update(context, 30);
        assert_eq!(estimator.get_k(context), 5);
    }
}
