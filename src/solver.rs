use std::{borrow::Cow, ops::Neg};

use once_cell::sync::OnceCell;

use crate::Correctness;

use super::{Guess, Guesser, DICTIONARY};

static INITIAL: OnceCell<Vec<(&'static str, usize)>> = OnceCell::new();

pub struct Solver {
    remaining: Cow<'static, Vec<(&'static str, usize)>>,
}

impl Solver {
    pub fn new() -> Self {
        Self {
            remaining: Cow::Borrowed(INITIAL.get_or_init(|| {
                Vec::from_iter(DICTIONARY.lines().map(|line| {
                    let (word, count) = line
                        .split_once(' ')
                        .expect("Every line is word + space + frequency");
                    let count: usize = count.parse().expect("every count is a number");
                    return (word, count);
                }))
            })),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Candidate {
    word: &'static str,
    goodness: f64,
}

impl Guesser for Solver {
    fn guess(&mut self, history: &[Guess]) -> String {
        if let Some(last) = history.last() {
            if matches!(self.remaining, Cow::Owned(_)) {
                self.remaining
                    .to_mut()
                    .retain(|(word, _)| last.matches(word));
            } else {
                self.remaining = Cow::Owned(
                    self.remaining
                        .iter()
                        .filter(|(word, _)| last.matches(word))
                        .copied()
                        .collect(),
                );
            }
        }

        if history.is_empty() {
            return "crate".to_string();
        }

        let remaining_count: usize = self.remaining.iter().map(|(_, c)| c).sum();
        let mut best: Option<Candidate> = None;
        for &(word, _) in &*self.remaining {
            let mut sum = 0.0;

            for pattern in Correctness::patterns() {
                let mut in_patter_total = 0;
                for (candidate, count) in &*self.remaining {
                    let g = Guess {
                        word: Cow::Borrowed(word),
                        mask: pattern,
                    };
                    if g.matches(candidate) {
                        in_patter_total += count;
                    }
                }

                if in_patter_total == 0 {
                    continue;
                }
                let p_of_this_pattern = in_patter_total as f64 / remaining_count as f64;
                sum += p_of_this_pattern * p_of_this_pattern.log2();
            }
            let goodness = sum.neg();
            if let Some(c) = best {
                if goodness > c.goodness {
                    best = Some(Candidate { word, goodness })
                }
            } else {
                best = Some(Candidate { word, goodness })
            }
        }
        best.unwrap().word.to_string()
    }
}
