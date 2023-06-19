use std::collections::HashSet;

/// Sort a vec in place, by providing a list of desired indices.
/// Thanks to: https://stackoverflow.com/a/69774341/5552584
pub fn sort_by_indices<T>(slice: &mut [T], mut indices: Vec<usize>) {
    // assert `indices` is valid input
    {
        let mut seen = HashSet::new();
        let len = slice.len();
        for idx in &indices {
            assert!(*idx < len, "indices contains out of bounds index: {}", idx);
            assert!(
                seen.insert(idx),
                "indices contained duplicate index: {}",
                idx
            );
        }

        assert_eq!(
            len,
            seen.len(),
            "indices must have the same length as input slice"
        );
    }

    // perform sort
    for idx in 0..slice.len() {
        if indices[idx] != idx {
            let mut current_idx = idx;
            loop {
                let target_idx = indices[current_idx];
                indices[current_idx] = current_idx;
                if indices[target_idx] == target_idx {
                    break;
                }

                slice.swap(current_idx, target_idx);
                current_idx = target_idx;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "indices contains out of bounds index: 10")]
    fn oob_indices() {
        let indices = vec![0, 1, 2, 10];
        let mut data = vec!["a", "b", "c", "d"];
        sort_by_indices(&mut data, indices);
    }

    #[test]
    #[should_panic(expected = "indices contained duplicate index: 1")]
    fn duplicate_indices() {
        let indices = vec![1, 2, 3, 1];
        let mut data = vec!["a", "b", "c", "d"];
        sort_by_indices(&mut data, indices);
    }

    #[test]
    #[should_panic(expected = "indices must have the same length as input slice")]
    fn shorter_indices() {
        let indices = vec![1, 2];
        let mut data = vec!["a", "b", "c", "d"];
        sort_by_indices(&mut data, indices);
    }

    #[test]
    #[should_panic(expected = "indices contains out of bounds index: 4")]
    fn longer_indices() {
        let indices = vec![3, 2, 1, 0, 4];
        let mut data = vec!["a", "b", "c", "d"];
        sort_by_indices(&mut data, indices);
    }

    #[test]
    fn non_copy_types() {
        let indices = vec![0, 2, 3, 1];
        let mut data: Vec<String> = vec![
            String::from("a"),
            String::from("b"),
            String::from("c"),
            String::from("d"),
        ];
        sort_by_indices(&mut data, indices);
        assert_eq!(data, &["a", "c", "d", "b"]);
    }

    /// test a bunch of random datasets to ensure it works for all configurations
    #[test]
    fn monkey_test() {
        use rand::distributions::{Distribution, Uniform};
        use rand::prelude::SliceRandom;

        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            // get a random length
            let len: usize = Uniform::from(0..10).sample(&mut rng);

            // create the expected sorted output
            let sorted = (0..len).collect::<Vec<usize>>();

            // create an input list that's randomly shuffled
            let mut input = sorted.clone();
            input.shuffle(&mut rng);

            // compute the indices needed to have this list sorted
            let mut indices = sorted.clone();
            indices.sort_by_key(|&i| &input[i]);

            // sort!
            sort_by_indices(&mut input, indices.clone());

            // did it work?
            assert_eq!(input, sorted);
        }
    }
}
