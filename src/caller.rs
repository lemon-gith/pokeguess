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

        let fn_main: fn() -> () = {
            let choice: u8 = {
                input.trim()
                     .parse()
                     .unwrap_or_else(|invalid| {
                        println!("invalid input: \"{}\"", invalid);
                        println!("defaulting to option 2...");
                        2
                     })
            };
            if let Some(fn_struct) = fn_opts.get(&choice) {
                fn_struct.main
            } else {
                || println!("That's not a valid option, please try again")
            }
        };

        fn_main();

        println!();
    }
}
