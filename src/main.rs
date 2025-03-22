const GAMES: &str = include_str!("../answers.txt");
use wordle::{solver::Solver, Wordle};

fn main() {
    let wordle = Wordle::new();
    for answer in GAMES.split_whitespace() {
        let guesser = Solver::new();
        wordle.play(answer, guesser);
    }

    println!("Hello, world!");
}
