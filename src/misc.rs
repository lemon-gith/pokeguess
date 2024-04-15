// not to be included in main, just to run here, as is


fn temp_formatter(temp: &str) -> Result<(bool, String), String>{
    if temp.is_empty(){
        return Err(String::from("oi, this is empty >:("));
    }
    
    use regex::Regex;
    let Ok(valid_temp) = Regex::new(r"[0-9]+°?[cCfF]") else {
        return Err(String::from("Regex failed to regex"));
    };

    if !valid_temp.is_match(temp) {
        return Err(format!("{temp} is not in valid temperature format"));
    }

    let (is_celsius, suffix_len): (bool, usize) = {
        let rev_temp: Vec<char> = temp.chars().rev().take(2).collect();
        match rev_temp[..] {
            ['C', '°'] => (true, 2),
            ['F', '°'] => (false, 2),
            ['C', sth] if sth.is_numeric() => (true, 1),
            ['F', sth] if sth.is_numeric() => (false, 1),
            _ => return Err(format!(
                "{temp} is somehow not in valid temperature format"
            )),
        }
    };

    let temp_str = {
        let char_num = temp.chars().count();
        temp.char_indices().map(|(idx, digit)|
            if idx < (char_num-suffix_len) { digit } else { 'x' }
        ).filter(|&digit| digit != 'x').collect::<String>()
    };

    return Ok((is_celsius, temp_str));
}


fn temp_converter(temp: &str) -> Result<String, String> {
    let (is_celsius, temp_val) = match temp_formatter(temp) {
        Ok(vals) => vals,
        Err(err) => return Err(err),
    };

    // safe to unwrap because of temp_formatter
    let temp_val: f64 = temp_val.parse().unwrap();

    let temp_val = {
        match is_celsius {
            true => temp_val*(9.0/5.0) + 32.0,
            false => (temp_val-32.0) * (5.0/9.0),
        }
    };

    let mut temp_string = temp_val.to_string();
    temp_string.push_str(if is_celsius {"°F"} else {"°C"});

    return Ok(temp_string);
}

/// a function that generates values of the fibonacci sequence\
/// yes, this is 0-indexed :p
fn fibonacci_generator(end_idx: u128) -> u128 {
    #[inline(always)]
    fn fib(prev_prev: u128, prev: u128) -> u128 { prev_prev + prev }

    let mut prev_fib = 0;
    let mut cur_fib = 1;
    for _ in 1..end_idx {
        let next_fib = fib(prev_fib, cur_fib);
        (prev_fib, cur_fib) = (cur_fib, next_fib);
    }
    return cur_fib;
}

enum FuncOptions {
    FibGen = 0,
    TempConv = 1,
}

// this is how I was calling it, with the functions 
//fn caller(function: fn(u128) -> u128, arg: u128) -> () {
pub fn caller() -> () {
    const FUNCTION_ARR: [(u8, &str); 3] = [
        (0, "fibonacci generator"),
        (1, "temperature converter"),
        (123, "exit")
    ];
    
    loop {
        println!("Please enter the index of the function to run:");

        for (idx, fn_name) in FUNCTION_ARR {
            println!("{:<3}: {}", idx, fn_name);
        }

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
