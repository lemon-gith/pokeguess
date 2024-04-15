// not to be included in main, just to run here, as is
#![allow(unused)]
/*
1. Convert temperatures between Fahrenheit and Celsius.
2. Generate the nth Fibonacci number.
 */

fn temp_converter(temp: &str) -> &str {
    "oof"
}

/// a function that generates values of the fibonacci sequence\
/// yes, this is 0-indexed :p
fn fibonacci_generator(end_idx: u64) -> u64 {
    #[inline(always)]
    fn fib(prev_prev: u64, prev: u64) -> u64 { prev_prev + prev }

    let mut prev_fib = 0;
    let mut cur_fib = 1;
    for _ in 1..end_idx {
        let next_fib = fib(prev_fib, cur_fib);
        (prev_fib, cur_fib) = (cur_fib, next_fib);
    }
    return cur_fib;
}

// TODO: extend this to offer options and be more interactive
// TODO: write the temp_converter...
// TODO: add timing: calls function multiple times and av. exec times
// this is how I was calling it, with the functions 
//fn caller(function: fn(u64) -> u64, arg: u64) -> () {
fn caller(functions: (fn(u64) -> u64, fn(&str) -> &str)) -> () {
    let mut exit: bool = false;
    while !exit {
        println!("Please pick a function to run (0 or 1):");
        
        println!("0: fibonacci generator");
        println!("1: temperature converter");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("hmmm");
        input = input.trim();

        let (function, fn_idx) = match input {
            "0" => (functions.0, "0"),
            "1" => (functions.1, "1"),
            _ => {
                println!("Oi, that's not a valid input :(");
                println!("selecting option 0...");
                (functions.0, "0")
            }
        };

        match fn_idx {
            "0" => print!("Please enter an argument here: "),
            "1" => print!("Please enter an argument here: ")
        }
        std::io::Write::flush(&mut std::io::stdout());
        /* 
         * can flush stdout using above, since print! doesn't (println! does)
         * my guess is, gets the flush fn from Write, applies to stdout stream
         */

        std::io::stdin().read_line(&mut input).expect("hmmm");
        match fn_idx {
            "0" => {
                let arg: u64 = input.trim().parse().expect("oi");
                print!("res: {}", function(arg));
            },
            "1" => {
                let arg = input.trim();
                print!("res: {}", function(arg));
            }
        }
    }
}

fn main(){ caller((fibonacci_generator, temp_converter)); }