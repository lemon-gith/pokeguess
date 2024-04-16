pub fn temp_formatter(temp: &str) -> Result<(bool, String), String>{
  if temp.is_empty(){
      return Err(String::from("oi, this is empty >:("));
  }
  
  use regex::Regex;
  let Ok(valid_temp) = Regex::new(r"^-?[0-9]+(\.[0-9]+)?°?[cCfF]$") else {
      return Err(String::from("Regex failed to regex"));
  };

  if !valid_temp.is_match(temp) {
      return Err(format!("{temp} is not in valid temperature format"));
  }

  let (is_celsius, suffix_len): (bool, usize) = {
      let rev_temp: Vec<char> = temp.chars()
                                    .rev()
                                    .take(2)
                                    .map(|val| val.to_ascii_uppercase())
                                    .collect();

      match rev_temp[..] {
          ['C', '°'] => (true, 2),
          ['F', '°'] => (false, 2),
          ['C', sth] if sth.is_numeric() => (true, 1),
          ['F', sth] if sth.is_numeric() => (false, 1),
          _ => return Err(format!(
              "Somehow, {temp} is not in valid temperature format"
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


pub fn temp_converter(temp: &str) -> Result<String, String> {
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
pub fn fibonacci_generator(end_idx: u8) -> Result<u128, String> {
    if end_idx > 186 {
        return Err(String::from("Output is too large for u128 D:"));
    }

    #[inline(always)]
    fn fib(prev_prev: u128, prev: u128) -> Option<u128> { 
        prev_prev.checked_add(prev)
    }

    let mut prev_fib = 0;
    let mut cur_fib = 1;
    for _ in 1..end_idx {
        let Some(next_fib) = fib(prev_fib, cur_fib) else {
            return Err(String::from("Output is too large for u128 D:"));
        };
        (prev_fib, cur_fib) = (cur_fib, next_fib);
    }
    return Ok(cur_fib);
}
