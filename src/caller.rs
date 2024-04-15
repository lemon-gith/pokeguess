// not to be included in main, just to run here, as is
mod game_of_guess;
mod tools;


enum FuncOptions {
    FibGen = 0,
    TempConv = 1,
}

const FUNCTION_ARR: [(u8, &str); 3] = [
    (0, "fibonacci generator"),
    (1, "temperature converter"),
    (123, "exit")
];

/// a function to call the functions given by the accompanying data and modules
pub fn caller() -> () {
    use tools::{fibonacci_generator, temp_converter};
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
            123 => {
                println!("very well, exiting program...");
                return;
            }
            _ => FuncOptions::TempConv
        };
        input.clear();
        println!();

        // Pre-amble for user
        match fn_idx {
            FuncOptions::FibGen => print!("Please enter a fibonacci index: "),
            FuncOptions::TempConv => print!("Please enter a temperature: "),
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
                let arg: u128 = input.trim().parse().expect("oi");
                println!("value: {}", fibonacci_generator(arg));
            },
            FuncOptions::TempConv => {
                let arg = input.trim();
                match temp_converter(arg) {
                    Ok(value) => println!("temp: {}", value),
                    Err(err) => println!("Error: {}", err),
                }
            }
        }
        println!();
    }
}
