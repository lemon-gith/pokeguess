// not to be included in main, just to run here, as is
mod game_of_guess;
mod tools;


/// struct to hold hard-coded details about user options
struct FunctionNames {
    name: &'static str,
    desc: &'static str,
    main: fn() -> (),
}

/// a function to call the functions given by the accompanying data and modules
pub fn caller() {
    use std::collections::HashMap;
    // Map from idx to details and caller for each user option
    let fn_opts: HashMap<u8, FunctionNames> = HashMap::from([
        (0, FunctionNames {
            name: "Fibonacci Generator",
            desc: "Generates values from the Fibonacci Sequence",
            main: || tools::fibonacci_main(),
        }),
        (1, FunctionNames {
            name: "Temperature Converter",
            desc: "tool for converting between Celsius and Fahrenheit",
            main: || tools::temp_conv_main(),
        }),
        (2, FunctionNames {
            name: "Guessing Game",
            desc: "pokemon-based guessing game :D",
            main: || game_of_guess::guessing_game(),
        }),
        (123, FunctionNames {
            name: "Exit",
            desc: "exit this program",
            main: || {
                println!("okee, bye :D");
                std::process::exit(0);
            },
        }),
    ]);
    // New things to be called can be added in here :p

    loop {
        println!("Please enter the index of the function to run:");

        // display function options
        for (idx, fn_name) in &fn_opts {
            // Note: these numbers must be adjusted for aesthetics
            // for any changes to fn_opts
            println!("{:>3}: {:>22} - {}", idx, fn_name.name, fn_name.desc);
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
