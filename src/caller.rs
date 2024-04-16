// not to be included in main, just to run here, as is
mod game_of_guess;
mod tools;


enum FuncOptions {
    FibGen = 0,
    TempConv = 1,
    GuessGame = 2,
}

const FUNCTION_ARR: [(u8, &str); 4] = [
    (0, "fibonacci generator"),
    (1, "temperature converter"),
    (2, "guessing game :0"),
    (123, "exit")
];

/// a function to call the functions given by the accompanying data and modules
pub fn caller() -> () {
    use tools::{fibonacci_generator, temp_converter};
    use game_of_guess::guessing_game;
    loop {
        println!("Please enter the index of the function to run:");

        // display function options
        for (idx, fn_name) in FUNCTION_ARR {
            println!("{:<3}: {}", idx, fn_name);
        }

        // Get user to select a function
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("hmmm");
        input = String::from(input.trim());

        let fn_idx = match input.parse().unwrap_or(0) {
            0 => FuncOptions::FibGen,
            1 => FuncOptions::TempConv,
            2 => FuncOptions::GuessGame,
            123 => {
                println!("very well, exiting program...");
                return;
            }
            _ => FuncOptions::GuessGame
        };
        input.clear();
        println!();

        // Pre-amble for user
        match fn_idx {
            FuncOptions::FibGen => print!("Please enter a fibonacci index (u8): "),
            FuncOptions::TempConv => print!("Please enter a temperature: "),
            FuncOptions::GuessGame => print!("Please pick a game style: ")
        }
        let _ = std::io::Write::flush(&mut std::io::stdout());
        /* 
         * can flush stdout using above, since print! doesn't (println! does);
         * my guess is, gets the flush fn from Write, applies to stdout stream

         * ignoring returned Result,
         * because consequence of not being flushed is a minor UX issue
         */

        // Function calling
        std::io::stdin().read_line(&mut input).expect("hmmm");
        match fn_idx {
            FuncOptions::FibGen => {
                let Ok(arg) = input.trim().parse::<u8>() else {
                    eprintln!("That's not a u8 integer :(");
                    continue;
                };
                match fibonacci_generator(arg) {
                    Ok(value) => println!("value: {}", value),
                    Err(err) => eprintln!("Error: {}", err),
                }
            },
            FuncOptions::TempConv => {
                let arg = input.trim();
                match temp_converter(arg) {
                    Ok(value) => println!("temp: {}", value),
                    Err(err) => eprintln!("Error: {}", err),
                }
            },
            FuncOptions::GuessGame => {
                let arg: u8 = input.trim().parse().expect("oi");
                guessing_game(arg);
            },
        }
        println!();
    }
}
