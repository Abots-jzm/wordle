#![allow(clippy::type_complexity)]
#![allow(clippy::blocks_in_if_conditions)]

use std::{borrow::Cow, collections::HashSet, num::NonZeroU8};

mod solver;
pub use solver::Solver;

include!(concat!(env!("OUT_DIR"), "/dictionary.rs"));

pub struct Wordle {
    dictionary: HashSet<&'static str>,
}

impl Default for Wordle {
    fn default() -> Self {
        Self::new()
    }
}

impl Wordle {
    pub fn new() -> Self {
        Self {
            dictionary: HashSet::from_iter(DICTIONARY.iter().copied().map(|(word, _)| word)),
        }
    }

    pub fn play(&self, answer: &'static str, mut guesser: Solver) -> Option<usize> {
        let mut history = Vec::new();
        // Wordle only allows six guesses.
        // We allow more to avoid chopping off the score distribution for stats purposes.
        for i in 1..=32 {
            let guess = guesser.guess(&history);
            if guess == answer {
                let correctness = Correctness::compute(answer, &guess);
                let current_guess = Guess {
                    word: Cow::Owned(guess),
                    mask: correctness,
                };
                Wordle::display_guess(&current_guess);
                return Some(i);
            }
            assert!(self.dictionary.contains(&*guess));
            let correctness = Correctness::compute(answer, &guess);
            let current_guess = Guess {
                word: Cow::Owned(guess),
                mask: correctness,
            };
            Wordle::display_guess(&current_guess);
            history.push(current_guess);
        }

        None
    }

    fn display_guess(guess: &Guess) {
        // Print the word with colored backgrounds according to the mask
        for (c, &correctness) in guess.word.chars().zip(&guess.mask) {
            let display = match correctness {
                Correctness::Correct => format!("\x1b[42m {c} \x1b[0m"), // Green background
                Correctness::Misplaced => format!("\x1b[43m {c} \x1b[0m"), // Yellow background
                Correctness::Wrong => format!("\x1b[40m {c} \x1b[0m"),   // Black background
            };
            print!("{display}");
        }
        println!();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Correctness {
    /// Green
    Correct,
    /// Yellow
    Misplaced,
    /// Gray
    Wrong,
}

impl Correctness {
    fn is_misplaced(letter: u8, answer: &str, used: &mut [bool; 5]) -> bool {
        answer.bytes().enumerate().any(|(i, a)| {
            if a == letter && !used[i] {
                used[i] = true;
                return true;
            }
            false
        })
    }

    pub fn compute(answer: &str, guess: &str) -> [Self; 5] {
        assert_eq!(answer.len(), 5);
        assert_eq!(guess.len(), 5);
        let mut c = [Correctness::Wrong; 5];
        let answer_bytes = answer.as_bytes();
        let guess_bytes = guess.as_bytes();
        // Array indexed by lowercase ascii letters
        let mut misplaced = [0u8; (b'z' - b'a' + 1) as usize];

        // Find all correct letters
        for ((&answer, &guess), c) in answer_bytes.iter().zip(guess_bytes).zip(c.iter_mut()) {
            if answer == guess {
                *c = Correctness::Correct
            } else {
                // If the letter does not match, count it as misplaced
                misplaced[(answer - b'a') as usize] += 1;
            }
        }
        // Check all of the non matching letters if they are misplaced
        for (&guess, c) in guess_bytes.iter().zip(c.iter_mut()) {
            // If the letter was guessed wrong and the same letter was counted as misplaced
            if *c == Correctness::Wrong && misplaced[(guess - b'a') as usize] > 0 {
                *c = Correctness::Misplaced;
                misplaced[(guess - b'a') as usize] -= 1;
            }
        }

        c
    }
}

pub const MAX_MASK_ENUM: usize = 3 * 3 * 3 * 3 * 3;

/// A wrapper type for `[Correctness; 5]` packed into a single byte with a niche.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
// The NonZeroU8 here lets the compiler know that we're not using the value `0`, and that `0` can
// therefore be used to represent `None` for `Option<PackedCorrectness>`.
struct PackedCorrectness(NonZeroU8);

impl From<[Correctness; 5]> for PackedCorrectness {
    fn from(c: [Correctness; 5]) -> Self {
        let packed = c.iter().fold(0, |acc, c| {
            acc * 3
                + match c {
                    Correctness::Correct => 0,
                    Correctness::Misplaced => 1,
                    Correctness::Wrong => 2,
                }
        });
        Self(NonZeroU8::new(packed + 1).unwrap())
    }
}

impl From<PackedCorrectness> for u8 {
    fn from(this: PackedCorrectness) -> Self {
        this.0.get() - 1
    }
}

pub struct Guess<'a> {
    pub word: Cow<'a, str>,
    pub mask: [Correctness; 5],
}

impl Guess<'_> {
    pub fn matches(&self, word: &str) -> bool {
        // Check if the guess would be possible to observe when `word` is the correct answer.
        // This is equivalent to
        //     Correctness::compute(word, &self.word) == self.mask
        // without _necessarily_ computing the full mask for the tested word
        assert_eq!(word.len(), 5);
        assert_eq!(self.word.len(), 5);
        let mut used = [false; 5];

        // Check Correct letters
        for (i, (a, g)) in word.bytes().zip(self.word.bytes()).enumerate() {
            if a == g {
                if self.mask[i] != Correctness::Correct {
                    return false;
                }
                used[i] = true;
            } else if self.mask[i] == Correctness::Correct {
                return false;
            }
        }

        // Check Misplaced letters
        for (g, e) in self.word.bytes().zip(self.mask.iter()) {
            if *e == Correctness::Correct {
                continue;
            }
            if Correctness::is_misplaced(g, word, &mut used) != (*e == Correctness::Misplaced) {
                return false;
            }
        }

        // The rest will be all correctly Wrong letters
        true
    }
}
