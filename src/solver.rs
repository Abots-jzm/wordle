use core::f64;
use std::borrow::Cow;

use once_cell::sync::OnceCell;

use crate::{enumerate_mask, Correctness, MAX_MASK_ENUM};

use super::{Guess, Guesser, DICTIONARY};

static INITIAL: OnceCell<Vec<(&'static str, f64)>> = OnceCell::new();

pub struct Solver {
    remaining: Cow<'static, Vec<(&'static str, f64)>>,
    entropy: Vec<f64>,
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

fn est_steps_left(entropy: f64) -> f64 {
    (entropy * 3.870 + 3.679).ln()
}

const L: f64 = 1.0;
// How steep is the cut-off?
const K: f64 = 30000000.0;
// Where is the cut-off?
const X0: f64 = 0.00000497;

fn sigmoid(p: f64) -> f64 {
    L / (1.0 + (-K * (p - X0)).exp())
}

impl Solver {
    pub fn new() -> Self {
        Self {
            remaining: Cow::Borrowed(INITIAL.get_or_init(|| {
                let mut sum = 0;
                let mut words = Vec::from_iter(DICTIONARY.lines().map(|line| {
                    let (word, count) = line
                        .split_once(' ')
                        .expect("Every line is word + space + frequency");
                    let count: usize = count.parse().expect("every count is a number");
                    sum += count;
                    (word, count)
                }));
                words.sort_unstable_by_key(|&(_, count)| std::cmp::Reverse(count));

                let words: Vec<_> = words
                    .into_iter()
                    .map(|(word, count)| (word, sigmoid(count as f64 / sum as f64)))
                    .collect();

                words
            })),
            entropy: Vec::new(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Candidate {
    word: &'static str,
    e_score: f64,
}

impl Guesser for Solver {
    fn guess(&mut self, history: &[Guess]) -> String {
        let score = history.len() as f64;

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
            return "tares".to_string();
        }

        let remaining_p: f64 = self.remaining.iter().map(|&(_, p)| p).sum();
        let remaining_entropy = -self
            .remaining
            .iter()
            .map(|&(_, p)| {
                let p = p / remaining_p;
                p * p.log2()
            })
            .sum::<f64>();
        self.entropy.push(remaining_entropy);

        let mut best: Option<Candidate> = None;
        for &(word, count) in &*self.remaining {
            let mut totals = [0.0f64; MAX_MASK_ENUM];
            for (candidate, count) in &*self.remaining {
                let idx = enumerate_mask(&Correctness::compute(candidate, word));
                totals[idx] += count;
            }

            let sum: f64 = totals
                .into_iter()
                .filter(|t| *t != 0.0)
                .map(|t| {
                    let p_of_this_pattern = t as f64 / remaining_p as f64;
                    p_of_this_pattern * p_of_this_pattern.log2()
                })
                .sum();

            let p_word = count as f64 / remaining_p as f64;
            let e_info = -sum;
            let e_score = p_word * (score + 1.0)
                + (1.0 - p_word) * (score + est_steps_left(remaining_entropy - e_info));
            if let Some(c) = best {
                if e_score < c.e_score {
                    best = Some(Candidate { word, e_score });
                }
            } else {
                best = Some(Candidate { word, e_score });
            }
        }
        best.unwrap().word.to_string()
    }
}
