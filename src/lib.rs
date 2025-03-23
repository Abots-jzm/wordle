use std::{borrow::Cow, collections::HashSet};

pub mod solver;
const DICTIONARY: &str = include_str!("../dictionary.txt");

pub struct Wordle {
    dictionary: HashSet<&'static str>,
}

impl Wordle {
    pub fn new() -> Self {
        Self {
            dictionary: HashSet::from_iter(DICTIONARY.lines().map(|line| {
                line.split_once(' ')
                    .expect("Every line is word + space + frequency")
                    .0
            })),
        }
    }

    pub fn play<G: Guesser>(&self, answer: &'static str, mut guesser: G) -> Option<usize> {
        //play six rounds where it invokes the guesser each round
        let mut history = Vec::new();
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
        let mut used = [false; 5];
        for (i, (a, g)) in answer.bytes().zip(guess.bytes()).enumerate() {
            if a == g {
                c[i] = Correctness::Correct;
                used[i] = true;
            }
        }

        for (i, g) in guess.bytes().enumerate() {
            if c[i] == Correctness::Correct {
                continue;
            }

            if Correctness::is_misplaced(g, answer, &mut used) {
                c[i] = Correctness::Misplaced;
            }
        }

        c
    }

    pub fn patterns() -> impl Iterator<Item = [Self; 5]> {
        itertools::iproduct!(
            [Self::Correct, Self::Misplaced, Self::Wrong],
            [Self::Correct, Self::Misplaced, Self::Wrong],
            [Self::Correct, Self::Misplaced, Self::Wrong],
            [Self::Correct, Self::Misplaced, Self::Wrong],
            [Self::Correct, Self::Misplaced, Self::Wrong]
        )
        .map(|(a, b, c, d, e)| [a, b, c, d, e])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Correctness {
    ///Green
    Correct,
    ///Yellow
    Misplaced,
    ///Gray
    Wrong,
}

pub fn enumerate_mask(c: &[Correctness; 5]) -> usize {
    c.iter().fold(0, |acc, c| {
        acc * 3
            + match c {
                Correctness::Correct => 0,
                Correctness::Misplaced => 1,
                Correctness::Wrong => 2,
            }
    })
}

pub const MAX_MASK_ENUM: usize = 3 * 3 * 3 * 3 * 3;

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

    pub fn compatible_pattern(&self, other_pattern: &[Correctness; 5]) -> bool {
        let mut self_wrong = 0;
        let mut self_correct = 0;
        for c in self.mask.iter() {
            match c {
                Correctness::Correct => self_correct += 1,
                Correctness::Wrong => self_wrong += 1,
                _ => {}
            }
        }

        let mut other_wrong = 0;
        let mut other_correct = 0;
        for c in other_pattern.iter() {
            match c {
                Correctness::Correct => other_correct += 1,
                Correctness::Wrong => other_wrong += 1,
                _ => {}
            }
        }

        other_wrong <= self_wrong && other_correct >= self_correct
    }
}

pub trait Guesser {
    fn guess(&mut self, history: &[Guess]) -> String;
}

#[cfg(test)]
macro_rules! mask {
    (C) => {
        $crate::Correctness::Correct
    };
    (M) => {
        $crate::Correctness::Misplaced
    };
    (W) => {
        $crate::Correctness::Wrong
    };
    ($($c:tt)+) => {
        [$(mask!($c)),*]
    };
}

#[cfg(test)]
mod tests {
    mod guess_matcher {
        use crate::Guess;
        use std::borrow::Cow;

        macro_rules! check {
            ($prev:literal + [$($mask:tt)+] allows $next:literal) => {
                assert!(Guess {
                    word: Cow::Borrowed($prev),
                    mask: mask![$($mask )+]
                }
                .matches($next));
                assert_eq!($crate::Correctness::compute($next, $prev), mask![$($mask )+]);
            };
            ($prev:literal + [$($mask:tt)+] disallows $next:literal) => {
                assert!(!Guess {
                    word: Cow::Borrowed($prev),
                    mask: mask![$($mask )+]
                }
                .matches($next));
                assert_ne!($crate::Correctness::compute($next, $prev), mask![$($mask )+]);
            }
        }

        #[test]
        fn matches() {
            check!("abcde" + [C C C C C] allows "abcde");
            check!("abcdf" + [C C C C C] disallows "abcde");
            check!("abcde" + [W W W W W] allows "fghij");
            check!("abcde" + [M M M M M] allows "eabcd");
            check!("aaabb" + [C M W W W] disallows "accaa");
            check!("baaaa" + [W C M W W] disallows "caacc");
            check!("baaaa" + [W C M W W] allows "aaccc");
            check!("abcde" + [W W W W W] disallows "bcdea");
        }
    }

    mod compute {
        use crate::Correctness;

        #[test]
        fn all_green() {
            assert_eq!(Correctness::compute("abcde", "abcde"), mask!(C C C C C ))
        }

        #[test]
        fn all_gray() {
            assert_eq!(Correctness::compute("abcde", "fghij"), mask!(W W W W W))
        }

        #[test]
        fn all_yellow() {
            assert_eq!(Correctness::compute("abcde", "bcdea"), mask!(M M M M M))
        }

        #[test]
        fn repeat_green() {
            assert_eq!(Correctness::compute("aabbb", "aaccc"), mask!(C C W W W))
        }

        #[test]
        fn repeat_yellow() {
            assert_eq!(Correctness::compute("aabbb", "ccaac"), mask!(W W M M W))
        }

        #[test]
        fn repeat_some_green() {
            assert_eq!(Correctness::compute("abcde", "aacde"), mask!(C W C C C))
        }
    }
}
