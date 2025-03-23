use core::f64;
use std::borrow::Cow;

use once_cell::sync::OnceCell;

use crate::{enumerate_mask, Correctness, MAX_MASK_ENUM};

use super::{Guess, Guesser, DICTIONARY};

static INITIAL: OnceCell<Vec<(&'static str, f64, usize)>> = OnceCell::new();
static PATTERNS: OnceCell<Vec<[Correctness; 5]>> = OnceCell::new();
static COMPUTES: OnceCell<(usize, Vec<OnceCell<u8>>)> = OnceCell::new();

pub struct Solver {
    remaining: Cow<'static, Vec<(&'static str, f64, usize)>>,
    patterns: Cow<'static, Vec<[Correctness; 5]>>,
    entropy: Vec<f64>,
    computes: &'static (usize, Vec<OnceCell<u8>>),
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
        let remaining = Cow::Borrowed(INITIAL.get_or_init(|| {
            let mut sum = 0;
            let mut words = Vec::from_iter(DICTIONARY.lines().map(|line| {
                let (word, count) = line
                    .split_once(' ')
                    .expect("every line is word + space + frequency");
                let count: usize = count.parse().expect("every count is a number");
                sum += count;
                (word, count)
            }));

            words.sort_unstable_by_key(|&(_, count)| std::cmp::Reverse(count));

            let words: Vec<_> = words
                .into_iter()
                .enumerate()
                .map(|(idx, (word, count))| (word, sigmoid(count as f64 / sum as f64), idx))
                .collect();

            words
        }));

        let dimension = remaining.len();

        Self {
            remaining,
            patterns: Cow::Borrowed(PATTERNS.get_or_init(|| Correctness::patterns().collect())),
            entropy: Vec::new(),
            computes: COMPUTES
                .get_or_init(|| (dimension, vec![Default::default(); dimension * dimension])),
        }
    }
}

fn cachable_enumeration(answer: &str, guess: &str) -> u8 {
    enumerate_mask(&Correctness::compute(answer, guess)) as u8
}

fn get_row(
    computes: &'static (usize, Vec<OnceCell<u8>>),
    guess_idx: usize,
) -> &'static [OnceCell<u8>] {
    let (dimension, vec) = computes;
    let start = guess_idx * dimension;
    let end = start + dimension;
    &vec[start..end]
}

fn get_enumeration(row: &[OnceCell<u8>], answer: &str, guess: &str, guess_idx: usize) -> u8 {
    *row[guess_idx].get_or_init(|| cachable_enumeration(guess, &answer))
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
            let reference = enumerate_mask(&last.mask) as u8;
            let last_idx = self
                .remaining
                .iter()
                .find(|(word, _, _)| &*last.word == *word)
                .unwrap()
                .2;
            let row = get_row(self.computes, last_idx);
            if matches!(self.remaining, Cow::Owned(_)) {
                self.remaining.to_mut().retain(|(word, _, word_idx)| {
                    reference == get_enumeration(row, &last.word, word, *word_idx)
                });
            } else {
                self.remaining = Cow::Owned(
                    self.remaining
                        .iter()
                        .filter(|(word, _, word_idx)| {
                            reference == get_enumeration(row, &last.word, word, *word_idx)
                        })
                        .copied()
                        .collect(),
                );
            }
        }
        if history.is_empty() {
            self.patterns = Cow::Borrowed(PATTERNS.get().unwrap());
            // NOTE: I did a manual run with this commented out and it indeed produced "tares" as
            // the first guess. It slows down the run by a lot though.
            return "tares".to_string();
        } else {
            assert!(!self.patterns.is_empty());
        }

        let remaining_p: f64 = self.remaining.iter().map(|&(_, p, _)| p).sum();
        let remaining_entropy = -self
            .remaining
            .iter()
            .map(|&(_, p, _)| {
                let p = p / remaining_p;
                p * p.log2()
            })
            .sum::<f64>();
        self.entropy.push(remaining_entropy);

        let mut best: Option<Candidate> = None;
        let mut i = 0;
        let stop = (self.remaining.len() / 3).max(20);
        for &(word, count, word_idx) in &*self.remaining {
            // considering a world where we _did_ guess `word` and got `pattern` as the
            // correctness. now, compute what _then_ is left.

            // Rather than iterate over the patterns sequentially and add up the counts of words
            // that result in that pattern, we can instead keep a running total for each pattern
            // simultaneously by storing them in an array. We can do this since each candidate-word
            // pair deterministically produces only one mask.
            let mut totals = [0.0f64; MAX_MASK_ENUM];
            let row = get_row(self.computes, word_idx);
            for (candidate, count, candidate_idx) in &*self.remaining {
                let idx = get_enumeration(row, &word, candidate, *candidate_idx);
                totals[idx as usize] += count;
            }

            let sum: f64 = totals
                .into_iter()
                .filter(|t| *t != 0.0)
                .map(|p| {
                    let p_of_this_pattern = p as f64 / remaining_p as f64;
                    p_of_this_pattern * p_of_this_pattern.log2()
                })
                .sum();

            let p_word = count as f64 / remaining_p as f64;
            let e_info = -sum;
            let e_score = p_word * (score + 1.0)
                + (1.0 - p_word) * (score + est_steps_left(remaining_entropy - e_info));
            if let Some(c) = best {
                // Which one gives us a lower (expected) score?
                if e_score < c.e_score {
                    best = Some(Candidate { word, e_score });
                }
            } else {
                best = Some(Candidate { word, e_score });
            }

            i += 1;
            if i >= stop {
                break;
            }
        }
        best.unwrap().word.to_string()
    }
}
