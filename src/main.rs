const GAMES: &str = include_str!("../answers.txt");
use wordle::{solver::Solver, Wordle};

fn main() {
    let wordle = Wordle::new();
    for answer in GAMES.split_whitespace() {
        let guesser = Solver::new();
        if let Some(score) = wordle.play(answer, guesser) {
            println!("guessed '{}' in {} tries", answer, score);
        } else {
            eprintln!("failed to guess");
        }
    }

    println!("Hello, world!");
}
