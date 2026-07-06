pub const SIP_SIZE: i16 = 27;
pub const SIP_PER_WORD: u32 = 3;
pub const WORD_SIZE: i16 = SIP_SIZE.pow(SIP_PER_WORD);
pub const HALF_WORD: i16 = WORD_SIZE / 2;
pub const RAM_SIZE: usize = WORD_SIZE as usize;
pub const STO_SIZE: usize = WORD_SIZE as usize;

pub fn char_to_digit(c: char) -> Option<i16> {
    match c {
        '.' => Some(0),
        'a'..='m' => Some((c as u8 - b'a' + 1) as i16),
        'n'..='z' => Some(c as i16 - b'n' as i16 - SIP_SIZE / 2),
        'A'..='M' => Some((c as u8 - b'A' + 1) as i16),
        'N'..='Z' => Some(c as i16 - b'N' as i16 - SIP_SIZE / 2),
        _ => None,
    }
}

pub fn digit_to_char(d: i16) -> Option<char> {
    match d {
        0 => Some('.'),
        1..=13 => Some((b'a' + (d as u8) - 1) as char),
        -13..=-1 => Some((b'n' + (d + SIP_SIZE / 2) as u8) as char),
        _ => None,
    }
}

pub fn i16_to_code(mut value: i16) -> String {
    let mut digits = Vec::new();

    while value != 0 {
        let mut rem = value % SIP_SIZE;
        value /= SIP_SIZE;
        if rem > SIP_SIZE / 2 {
            rem -= SIP_SIZE;
            value += 1;
        } else if rem < -SIP_SIZE / 2 {
            rem += SIP_SIZE;
            value -= 1;
        }
        digits.insert(0, rem);
    }

    let mut code = String::new();
    for d in digits {
        code.push(digit_to_char(d).unwrap_or('?'));
    }

    while code.len() < SIP_PER_WORD as usize {
        code.insert(0, '.');
    }

    code
}

pub fn code_to_i16(token: &str) -> Result<i16, String> {
    let mut value = 0i16;
    if token.chars().all(|c| c == '.' || c.is_ascii_alphabetic()) {
        for char in token.chars() {
            let d = char_to_digit(char).ok_or_else(|| format!("Invalid character: {}", char))?;
            value = value * SIP_SIZE + d;
        }
    } else {
        value = token
            .parse::<i16>()
            .map_err(|_| format!("Invalid number: {}", token))?
    }

    if value < -(HALF_WORD) || value > HALF_WORD {
        Err(format!("Value out of word range: {}", value))
    } else {
        Ok(value)
    }
}

pub fn i16_to_trit(mut n: i16) -> Vec<i8> {
    let mut trits = Vec::new();

    if n == 0 {
        trits.push(0);
        return trits;
    }

    while n != 0 {
        let mut rem = n % 3;
        n /= 3;

        match rem {
            2 => {
                rem = -1;
                n += 1;
            }
            -2 => {
                rem = 1;
                n -= 1;
            }
            _ => {}
        }

        trits.push(rem as i8);
    }

    trits
}

pub fn trit_to_i16(trits: &[i8]) -> i16 {
    let mut value: i16 = 0;
    let mut place: i16 = 1;

    for &t in trits {
        value += (t as i16) * place;
        place *= 3;
    }

    value
}
