use crate::rational::Rational;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct NormalizedEquation {
    pub terms: BTreeMap<String, Rational>,
    pub constant: Rational,
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(i128),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Equals,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let len = chars.len();

    while i < len {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        match c {
            '+' => { tokens.push(Token::Plus); i += 1; }
            '-' => { tokens.push(Token::Minus); i += 1; }
            '*' => { tokens.push(Token::Star); i += 1; }
            '/' => { tokens.push(Token::Slash); i += 1; }
            '=' => { tokens.push(Token::Equals); i += 1; }
            _ => {
                if c.is_ascii_digit() {
                    let start = i;
                    while i < len && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    let num: i128 = input[start..i].parse().map_err(|_| format!("Ungültige Zahl: {}", &input[start..i]))?;
                    tokens.push(Token::Number(num));
                } else if c.is_alphabetic() {
                    let start = i;
                    while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    tokens.push(Token::Ident(input[start..i].to_string()));
                } else {
                    return Err(format!("Unerwartetes Zeichen: '{}'", c));
                }
            }
        }
    }

    Ok(tokens)
}

#[derive(Clone, Debug)]
enum Atom {
    Num(Rational),
    VarTerm(String, Rational),
}

fn parse_factor(tokens: &[Token], pos: &mut usize) -> Result<Atom, String> {
    if *pos >= tokens.len() {
        return Err("Unerwartetes Ende des Ausdrucks".to_string());
    }

    match &tokens[*pos] {
        Token::Number(n) => {
            let num = *n;
            *pos += 1;
            if *pos < tokens.len() && matches!(tokens[*pos], Token::Ident(_)) {
                if let Token::Ident(name) = &tokens[*pos] {
                    *pos += 1;
                    Ok(Atom::VarTerm(name.clone(), Rational::from_int(num)))
                } else {
                    unreachable!()
                }
            } else {
                Ok(Atom::Num(Rational::from_int(num)))
            }
        }
        Token::Ident(name) => {
            *pos += 1;
            Ok(Atom::VarTerm(name.clone(), Rational::one()))
        }
        _ => Err(format!("Zahl oder Variable erwartet, erhalten: {:?}", tokens[*pos])),
    }
}

fn parse_term(tokens: &[Token], pos: &mut usize) -> Result<Atom, String> {
    let mut atom = parse_factor(tokens, pos)?;

    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Star => {
                *pos += 1;
                let right = parse_factor(tokens, pos)?;
                atom = match (atom, right) {
                    (Atom::Num(a), Atom::Num(b)) => Atom::Num(a * b),
                    (Atom::Num(a), Atom::VarTerm(v, b)) => Atom::VarTerm(v, a * b),
                    (Atom::VarTerm(v, a), Atom::Num(b)) => Atom::VarTerm(v, a * b),
                    (Atom::VarTerm(_, _), Atom::VarTerm(_, _)) => {
                        return Err("Nicht-linearer Term: Variable multipliziert mit Variable".to_string());
                    }
                };
            }
            Token::Slash => {
                *pos += 1;
                let right = parse_factor(tokens, pos)?;
                atom = match (atom, right) {
                    (Atom::Num(a), Atom::Num(b)) => Atom::Num(a / b),
                    (Atom::VarTerm(v, a), Atom::Num(b)) => Atom::VarTerm(v, a / b),
                    (Atom::Num(a), Atom::VarTerm(v, b)) => {
                        Atom::VarTerm(v, a / b)
                    }
                    (Atom::VarTerm(_, _), Atom::VarTerm(_, _)) => {
                        return Err("Nicht-linearer Term: Variable geteilt durch Variable".to_string());
                    }
                };
            }
            _ => break,
        }
    }

    Ok(atom)
}

fn parse_expression(tokens: &[Token], pos: &mut usize) -> Result<(BTreeMap<String, Rational>, Rational), String> {
    let mut terms: BTreeMap<String, Rational> = BTreeMap::new();
    let mut constant = Rational::zero();

    let first_sign = if *pos < tokens.len() && matches!(tokens[*pos], Token::Minus) {
        *pos += 1;
        -1
    } else {
        1
    };

    let atom = parse_term(tokens, pos)?;
    match atom {
        Atom::Num(n) => constant = Rational::new(first_sign * n.num, n.den),
        Atom::VarTerm(v, c) => {
            let coeff = Rational::new(first_sign * c.num, c.den);
            terms.insert(v, coeff);
        }
    }

    while *pos < tokens.len() {
        let sign = match &tokens[*pos] {
            Token::Plus => {
                *pos += 1;
                1i128
            }
            Token::Minus => {
                *pos += 1;
                -1i128
            }
            _ => break,
        };

        let atom = parse_term(tokens, pos)?;
        match atom {
            Atom::Num(n) => {
                let add = Rational::new(sign * n.num, n.den);
                constant = constant + add;
            }
            Atom::VarTerm(v, c) => {
                let coeff = Rational::new(sign * c.num, c.den);
                *terms.entry(v).or_insert(Rational::zero()) = terms.get(&v).cloned().unwrap_or(Rational::zero()) + coeff;
            }
        }
    }

    Ok((terms, constant))
}

pub fn parse_equation(input: &str) -> Result<NormalizedEquation, String> {
    let tokens = tokenize(input)?;

    let mut pos = 0;
    let (left_terms, left_const) = parse_expression(&tokens, &mut pos)?;

    if pos >= tokens.len() || !matches!(tokens[pos], Token::Equals) {
        return Err("'=' in der Gleichung erwartet".to_string());
    }
    pos += 1;

    let (right_terms, right_const) = parse_expression(&tokens, &mut pos)?;

    if pos < tokens.len() {
        return Err(format!("Unerwartetes Token nach Gleichung: {:?}", tokens[pos]));
    }

    let mut all_terms: BTreeMap<String, Rational> = left_terms.clone();
    for (var, coeff) in &right_terms {
        let entry = all_terms.entry(var.clone()).or_insert(Rational::zero());
        *entry = entry.clone() - coeff.clone();
    }

    let constant = right_const - left_const;

    let mut cleaned: BTreeMap<String, Rational> = BTreeMap::new();
    for (var, coeff) in &all_terms {
        if !coeff.is_zero() {
            cleaned.insert(var.clone(), coeff.clone());
        }
    }

    Ok(NormalizedEquation {
        terms: cleaned,
        constant,
    })
}
