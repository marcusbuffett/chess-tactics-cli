use std::env;
#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate prettytable;

use anyhow::Context;
use clap::{AppSettings, Clap};
use colored::*;
use serde::{Deserialize, Serialize};
use shakmaty::{
    fen::{self, Fen},
    san::{self, San},
    uci::Uci,
    Board, CastlingMode, Chess, Color, Move, Piece, Position, Role, Setup, Square,
};

use anyhow::Result;

#[derive(Clap, Debug)]
#[clap(version = "1.0", author = "Marcus B. <me@mbuffett.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Args {
    #[clap(short, long)]
    /// The rating range of the tactics to fetch. Try 0-1200 for easy, 1200-1800 for
    /// intermediate, or 1800-3000 for difficult tactics.
    rating: Option<String>,
    #[clap(short, long)]
    /// Optionally specify a list of tags to get tactics for. Every tactic returned will have one
    /// of these tags
    tags: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Args::parse();
    // dbg!(&opts);
    let (rating_lower_bound, rating_upper_bound): (Option<i32>, Option<i32>) = {
        match opts.rating {
            Some(rating) => {
                let parts = rating.split("-").collect::<Vec<&str>>();
                match (parts.get(0), parts.get(1)) {
                    (Some(first), Some(second)) => {
                        let parse_rating = |s: &str| -> i32 {
                            s.parse::<i32>()
                                .expect(&format!("Failed to parse {} as a rating", s))
                        };
                        (Some(parse_rating(first)), Some(parse_rating(second)));
                    }
                    _ => {
                        panic!("Could not parse rating, make sure it's in the form '500-1200'")
                    }
                };
                (Some(0), Some(0))
            }
            None => (None, None),
        }
    };
    let tactic = get_new_puzzle(ChessTacticRequest {
        rating_gte: rating_lower_bound,
        rating_lte: rating_upper_bound,
        tags: opts.tags,
    })
    .await
    .expect("Failed to get a new tactic from the server, exiting.");
    let fen = tactic.fen;
    // let fen = "r6k/pp2r2p/4Rp1Q/3p4/8/1N1P2R1/PqP2bPP/7K b - - 0 24";
    let moves = tactic.moves;
    let setup: Fen = fen.parse()?;
    let mut position: Chess = setup.position(CastlingMode::Standard)?;
    let mut continuation_moves = moves.iter().map(|m| -> Uci { m.parse().unwrap() });
    let first_move = &continuation_moves
        .next()
        .unwrap()
        .to_move(&position)
        .unwrap();
    let their_side = position.turn();
    position = position.play(first_move).unwrap();
    println!();
    print_board(&position);
    let mut next_move = continuation_moves
        .next()
        .unwrap()
        .to_move(&position)
        .unwrap();
    loop {
        println!();
        let san_move = San::from_move(&position, &next_move);
        // dbg!(&san_move.to_string());
        let reply = get_prompt_response(&position);
        println!();
        let mut correct = false;
        match reply {
            PromptResponse::ShowBoard => {
                print_board(&position);
                continue;
            }
            PromptResponse::Help => {
                print_help();
                continue;
            }
            PromptResponse::PrintFen => {
                println!("{}", fen::epd(&position).to_string());
                continue;
            }
            PromptResponse::NoResponse => {}
            PromptResponse::HintPiece => {}
            PromptResponse::Move(move_input) => {
                if move_input == san_move.to_string() {
                    correct = true;
                } else {
                    println!("{} is not the correct move", move_input);
                    continue;
                }
            }
        }
        let reply = san_move.to_move(&position).unwrap();
        let old_position = position.clone();
        position = position.play(&reply).unwrap();
        let response = continuation_moves.next();
        match response {
            Some(response) => {
                let prefix = if correct {
                    "Correct! ".to_string()
                } else {
                    format!("The correct move was {}. ", san_move.to_string())
                };
                let response = response.to_move(&position).unwrap();
                let response_san = San::from_move(&position, &response);
                println!(
                    "{}{} responds with {}",
                    prefix,
                    print_side(&their_side),
                    response_san.to_string()
                );
                position = position.play(&response).unwrap();
                next_move = continuation_moves
                    .next()
                    .unwrap()
                    .to_move(&position)
                    .unwrap();
            }
            None => {
                let prefix = if correct {
                    "Correct! ".to_string()
                } else {
                    "".to_string()
                };
                println!("{}Completed this tactic.", prefix);
                break;
            }
        };
    }
    return Ok(());
}

enum PromptResponse {
    ShowBoard,
    NoResponse,
    PrintFen,
    Help,
    HintPiece,
    Move(String),
}

fn get_prompt_response(position: &Chess) -> PromptResponse {
    let reply = rprompt::prompt_reply_stdout(&get_prompt(position)).unwrap();
    match reply.as_ref() {
        "s" | "show" => return PromptResponse::ShowBoard,
        "f" | "fen" => return PromptResponse::PrintFen,
        "?" | "help" => return PromptResponse::Help,
        // "h" | "hint" => return PromptResponse::NoResponse,
        "" => return PromptResponse::NoResponse,
        x => return PromptResponse::Move(x.to_string()),
    }
}

#[derive(Deserialize, Debug)]
pub struct ChessTactic {
    pub id: String,
    pub moves: Vec<String>,
    pub fen: String,
    pub popularity: i32,
    pub tags: Vec<String>,
    pub game_link: String,
    pub rating: i32,
    pub rating_deviation: i32,
    pub number_plays: i32,
}

#[derive(Serialize, Debug)]
struct ChessTacticRequest {
    rating_gte: Option<i32>,
    rating_lte: Option<i32>,
    tags: Vec<String>,
}

async fn get_new_puzzle(request: ChessTacticRequest) -> Result<ChessTactic> {
    let client = reqwest::Client::new();
    let tactic: ChessTactic = client
        .post(get_api_endpoint())
        .header("User-Agent", "tactics-trainer-cli")
        .json(&request)
        .send()
        .await?
        .json()
        .await?;
    // dbg!(&tactic);
    return Ok(tactic);
}

fn get_api_endpoint() -> String {
    return format!(
        "{}/api/v1/tactic",
        env::var("TACTICS_SERVER_URL").unwrap_or("https://tactics.exoapi.app".to_string())
    );
}

fn print_side(side: &Color) -> String {
    if side == &Color::White {
        "White".to_string()
    } else {
        "Black".to_string()
    }
}

fn get_prompt(position: &Chess) -> String {
    let side = if position.turn() == Color::White {
        "White"
    } else {
        "Black"
    };
    return format!("{} to move, enter the best move, or '?' for help: ", side);
}

fn print_help() {
    ptable!(
        [
            "Any move, ex. Qxd7",
            "Attempt to solve the tactic with the given move"
        ],
        [
            "No input",
            "Reveal the answer, and continue the tactic if there are more moves."
        ],
        [
            "'f' or 'fen'",
            "Print out the current board, in FEN notation"
        ],
        ["'s' or 'show'", "Show the current board."],
        ["'?' or 'help'", "Display this help"]
    );
}

fn print_board(position: &Chess) {
    let board: &Board = position.board();
    for row in 0..8 {
        print!("  {}  ", 8 - row);
        for col in 0..8 {
            let idx = 64 - (row + 1) * 8 + col;
            // dbg!(idx);
            let square = Square::new(idx);
            // dbg!(square);
            let piece = board.piece_at(square);
            let square_is_white = (row + col) % 2 == 0;
            let c = if square_is_white { 140 } else { 80 };
            let piece_char = piece
                .map(|p: Piece| {
                    let ch = piece_ascii(&p);
                    let ch = if p.color == Color::White {
                        ch.blue()
                    } else {
                        ch.red()
                    };
                    ch
                })
                .unwrap_or("·".to_string().truecolor(c, c, c));
            if square_is_white {
                print!("{} ", piece_char);
            } else {
                print!("{} ", piece_char);
            }
        }
        println!();
    }

    println!(
        "     {}",
        (b'a'..=b'h')
            .map(char::from)
            .map(|c| c.to_string())
            .collect::<Vec<String>>()
            .join(" ")
    )
}

fn piece_unicode(piece: &Piece) -> &'static str {
    match (piece.role, piece.color) {
        (shakmaty::Role::Pawn, shakmaty::Color::Black) => "♟︎",
        (shakmaty::Role::Pawn, shakmaty::Color::White) => "♟︎",
        (shakmaty::Role::Knight, shakmaty::Color::Black) => "♞",
        (shakmaty::Role::Knight, shakmaty::Color::White) => "♞",
        (shakmaty::Role::Bishop, shakmaty::Color::Black) => "♝",
        (shakmaty::Role::Bishop, shakmaty::Color::White) => "♝",
        (shakmaty::Role::Rook, shakmaty::Color::Black) => "♜",
        (shakmaty::Role::Rook, shakmaty::Color::White) => "♜",
        (shakmaty::Role::Queen, shakmaty::Color::Black) => "♛",
        (shakmaty::Role::Queen, shakmaty::Color::White) => "♛",
        (shakmaty::Role::King, shakmaty::Color::Black) => "♚",
        (shakmaty::Role::King, shakmaty::Color::White) => "♚",
    }
}

fn piece_ascii(piece: &Piece) -> String {
    if piece.role == Role::Pawn {
        return match piece.color {
            Color::Black => "▲",
            Color::White => "▲",
        }
        .to_string();
    }
    return piece.char().to_uppercase().to_string();
    // match (piece.role, piece.color) {
    // (shakmaty::Role::Pawn, shakmaty::Color::Black) => {"♟︎"}
    // (shakmaty::Role::Pawn, shakmaty::Color::White) => {"♟︎"}
    // (shakmaty::Role::Knight, shakmaty::Color::Black) => {"♞"}
    // (shakmaty::Role::Knight, shakmaty::Color::White) => {"♞"}
    // (shakmaty::Role::Bishop, shakmaty::Color::Black) => {"♝"}
    // (shakmaty::Role::Bishop, shakmaty::Color::White) => {"♝"}
    // (shakmaty::Role::Rook, shakmaty::Color::Black) => {"♜"}
    // (shakmaty::Role::Rook, shakmaty::Color::White) => {"♜"}
    // (shakmaty::Role::Queen, shakmaty::Color::Black) => {"♛"}
    // (shakmaty::Role::Queen, shakmaty::Color::White) => {"♛"}
    // (shakmaty::Role::King, shakmaty::Color::Black) => {"♚"}
    // (shakmaty::Role::King, shakmaty::Color::White) => {"♚"}
    // }
}
