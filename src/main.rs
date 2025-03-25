use std::borrow::Cow;

use clap::Parser;
use wordle::Solver;

const GAMES: &str = include_str!("../answers.txt");

#[global_allocator]
static GLOBAL_ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    hard: bool,

    #[clap(short, long, conflicts_with = "all")]
    games: Option<usize>,

    #[clap(short, long, conflicts_with = "games")]
    all: bool,
}

fn main() {
    let args = Args::parse();

    let mut hard_mode = false;
    if args.hard {
        hard_mode = true;
    }

    if args.all {
        play(hard_mode, args.games);
    } else {
        play_interactive(hard_mode);
    }
}

fn play_interactive(hard_mode: bool) {
    let mut guesser = Solver::new(hard_mode);
    let mut history = Vec::with_capacity(6);
    println!("C: Correct / Green, M: Misplaced / Yellow, W: Wrong / Gray");
    // Wordle only allows six guesses.
    for _ in 1..=6 {
        let guess = guesser.guess(&history);
        println!("Guess:  {}", guess.to_uppercase());
        let correctness = {
            loop {
                match ask_for_correctness() {
                    Ok(c) => break c,
                    Err(e) => println!("{}", e),
                }
            }
        };
        if correctness == [wordle::Correctness::Correct; 5] {
            println!("The answer was {}", guess.to_uppercase());
            return;
        }
        history.push(wordle::Guess {
            word: Cow::Owned(guess),
            mask: correctness,
        });
    }
    println!("Game Over, only six guesses are allowed");
}

fn ask_for_correctness() -> Result<[wordle::Correctness; 5], Cow<'static, str>> {
    print!("Colors: ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    let mut answer = String::with_capacity(7);
    std::io::stdin().read_line(&mut answer).unwrap();
    let answer = answer
        .trim()
        .chars()
        .filter(|v| !v.is_whitespace())
        .map(|v| v.to_ascii_uppercase())
        .collect::<String>();
    if answer.len() != 5 {
        Err("You did not provide exactly 5 colors.")?;
    }
    let parsed = answer
        .chars()
        .map(|c| match c {
            'C' => Ok(wordle::Correctness::Correct),
            'M' => Ok(wordle::Correctness::Misplaced),
            'W' => Ok(wordle::Correctness::Wrong),
            _ => Err(format!(
                "The guess color '{c}' wasn't recognized: use C/M/W"
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(parsed
        .try_into()
        .expect("The parsed correctness is checked to be 5 items long"))
}

fn play(hard_mode: bool, max: Option<usize>) {
    let w = wordle::Wordle::new();
    let mut score = 0;
    let mut games = 0;
    let mut histogram = Vec::new();
    for answer in GAMES.split_whitespace().take(max.unwrap_or(usize::MAX)) {
        let guesser = Solver::new(hard_mode);
        if let Some(s) = w.play(answer, guesser) {
            println!("guessed '{}' in {} tries\n", answer, s);
            games += 1;
            score += s;
            if s >= histogram.len() {
                histogram.extend(std::iter::repeat(0).take(s - histogram.len() + 1));
            }
            histogram[s] += 1;
            // eprintln!("guessed '{}' in {}", answer, s);
        } else {
            eprintln!("failed to guess '{}'", answer);
        }
    }
    let sum: usize = histogram.iter().sum();
    for (score, count) in histogram.into_iter().enumerate().skip(1) {
        let frac = count as f64 / sum as f64;
        let w1 = (30.0 * frac).round() as usize;
        let w2 = (30.0 * (1.0 - frac)).round() as usize;
        eprintln!(
            "{:>2}: {}{} ({})",
            score,
            "#".repeat(w1),
            " ".repeat(w2),
            count
        );
    }
    eprintln!("average score: {:.4}", score as f64 / games as f64);
}
