//! The formula controller's expression language: numbers, `t` (song
//! position in beats), `bar` (`t / 4`) and `pi`; `+ - * / %`, parentheses
//! and unary minus; and the functions `sin cos abs floor frac sqrt min max
//! pow saw tri sqr`. [`Formula::parse`] compiles text into a fixed-size
//! program (main thread); [`Formula::eval`] runs it without allocating,
//! locking or panicking, so the audio thread can call it.
//!
//! Wavetable formulas ([`Formula::parse_table`], [`Formula::eval_table`])
//! use the variables `x` (phase within a frame) and `y` (position across the
//! frames) instead of `t` and `bar`.

/// Most tokens (and so program steps) in one formula.
pub const MAX_TOKENS: usize = 64;
/// Deepest value stack a formula may need.
const MAX_STACK: usize = 32;

/// One step of a compiled formula.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Num(f32),
    T,
    Bar,
    X,
    Y,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Call(Func),
}

/// A built-in function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Func {
    Sin,
    Cos,
    Abs,
    Floor,
    Frac,
    Sqrt,
    Min,
    Max,
    Pow,
    Saw,
    Tri,
    Sqr,
}

impl Func {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "abs" => Self::Abs,
            "floor" => Self::Floor,
            "frac" => Self::Frac,
            "sqrt" => Self::Sqrt,
            "min" => Self::Min,
            "max" => Self::Max,
            "pow" => Self::Pow,
            "saw" => Self::Saw,
            "tri" => Self::Tri,
            "sqr" => Self::Sqr,
            _ => return None,
        })
    }

    fn arity(self) -> usize {
        match self {
            Self::Min | Self::Max | Self::Pow => 2,
            _ => 1,
        }
    }
}

/// A compiled formula: a program in reverse Polish order. Plain `Copy`
/// data, so it can travel to the audio thread inside other messages.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Formula {
    ops: [Op; MAX_TOKENS],
    len: usize,
}

impl Default for Formula {
    /// The formula `0`.
    fn default() -> Self {
        Self {
            ops: [Op::Num(0.0); MAX_TOKENS],
            len: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Num(f32),
    Name(Func),
    T,
    Bar,
    X,
    Y,
    Op(char),
    Open,
    Close,
    Comma,
}

fn tokenize(text: &str, table: bool) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let value = word
                .parse::<f32>()
                .map_err(|_| format!("`{word}` is not a number"))?;
            tokens.push(Token::Num(value));
        } else if c.is_ascii_alphabetic() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            tokens.push(match word.as_str() {
                "t" if !table => Token::T,
                "bar" if !table => Token::Bar,
                "x" if table => Token::X,
                "y" if table => Token::Y,
                "pi" => Token::Num(std::f32::consts::PI),
                name => {
                    Token::Name(Func::parse(name).ok_or_else(|| format!("unknown name `{name}`"))?)
                }
            });
        } else {
            tokens.push(match c {
                '+' | '-' | '*' | '/' | '%' => Token::Op(c),
                '(' => Token::Open,
                ')' => Token::Close,
                ',' => Token::Comma,
                _ => return Err(format!("unexpected `{c}`")),
            });
            i += 1;
        }
    }
    if tokens.len() > MAX_TOKENS {
        return Err(format!("too long: at most {MAX_TOKENS} tokens"));
    }
    Ok(tokens)
}

/// A recursive-descent parser producing reverse Polish order.
struct Parser {
    tokens: Vec<Token>,
    at: usize,
    out: Vec<Op>,
}

impl Parser {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.at).copied()
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.peek();
        self.at += 1;
        token
    }

    fn expect(&mut self, want: Token, what: &str) -> Result<(), String> {
        match self.next() {
            Some(token) if token == want => Ok(()),
            _ => Err(format!("expected {what}")),
        }
    }

    /// sum := product (('+' | '-') product)*
    fn sum(&mut self) -> Result<(), String> {
        self.product()?;
        while let Some(Token::Op(c @ ('+' | '-'))) = self.peek() {
            self.at += 1;
            self.product()?;
            self.out.push(if c == '+' { Op::Add } else { Op::Sub });
        }
        Ok(())
    }

    /// product := unary (('*' | '/' | '%') unary)*
    fn product(&mut self) -> Result<(), String> {
        self.unary()?;
        while let Some(Token::Op(c @ ('*' | '/' | '%'))) = self.peek() {
            self.at += 1;
            self.unary()?;
            self.out.push(match c {
                '*' => Op::Mul,
                '/' => Op::Div,
                _ => Op::Mod,
            });
        }
        Ok(())
    }

    /// unary := '-' unary | atom
    fn unary(&mut self) -> Result<(), String> {
        if self.peek() == Some(Token::Op('-')) {
            self.at += 1;
            self.unary()?;
            self.out.push(Op::Neg);
            return Ok(());
        }
        self.atom()
    }

    /// atom := number | t | bar | '(' sum ')' | name '(' sum (',' sum)* ')'
    fn atom(&mut self) -> Result<(), String> {
        match self.next() {
            Some(Token::Num(v)) => self.out.push(Op::Num(v)),
            Some(Token::T) => self.out.push(Op::T),
            Some(Token::Bar) => self.out.push(Op::Bar),
            Some(Token::X) => self.out.push(Op::X),
            Some(Token::Y) => self.out.push(Op::Y),
            Some(Token::Open) => {
                self.sum()?;
                self.expect(Token::Close, "`)`")?;
            }
            Some(Token::Name(func)) => {
                self.expect(Token::Open, "`(` after a function name")?;
                for arg in 0..func.arity() {
                    if arg > 0 {
                        self.expect(Token::Comma, "`,` between arguments")?;
                    }
                    self.sum()?;
                }
                self.expect(Token::Close, "`)`")?;
                self.out.push(Op::Call(func));
            }
            _ => return Err("expected a number, a variable, `(` or a function".to_owned()),
        }
        Ok(())
    }
}

impl Formula {
    /// Compiles `text`. Main thread only (it allocates while parsing).
    ///
    /// # Errors
    ///
    /// Describes the first problem: an unknown name, a bad number, a
    /// missing parenthesis, a formula too long or too deeply nested.
    pub fn parse(text: &str) -> Result<Self, String> {
        Self::compile(text, false)
    }

    /// Compiles a wavetable formula of `x` and `y` (no `t` or `bar`). Main
    /// thread only.
    ///
    /// # Errors
    ///
    /// As [`Formula::parse`].
    pub fn parse_table(text: &str) -> Result<Self, String> {
        Self::compile(text, true)
    }

    fn compile(text: &str, table: bool) -> Result<Self, String> {
        let mut parser = Parser {
            tokens: tokenize(text, table)?,
            at: 0,
            out: Vec::new(),
        };
        if parser.tokens.is_empty() {
            return Err("the formula is empty".to_owned());
        }
        parser.sum()?;
        if parser.at != parser.tokens.len() {
            return Err("unexpected text after the formula".to_owned());
        }
        if parser.out.len() > MAX_TOKENS {
            return Err(format!("too long: at most {MAX_TOKENS} steps"));
        }
        // Check the stack depth once, so evaluation never overflows.
        let mut depth = 0_usize;
        for op in &parser.out {
            depth = match op {
                Op::Num(_) | Op::T | Op::Bar | Op::X | Op::Y => depth + 1,
                Op::Neg => depth,
                Op::Call(f) if f.arity() == 1 => depth,
                _ => depth.saturating_sub(1),
            };
            if depth > MAX_STACK {
                return Err("the formula is nested too deeply".to_owned());
            }
        }
        let mut ops = [Op::Num(0.0); MAX_TOKENS];
        for (slot, op) in ops.iter_mut().zip(&parser.out) {
            *slot = *op;
        }
        Ok(Self {
            ops,
            len: parser.out.len(),
        })
    }

    /// Evaluates at song position `t` beats. Non-finite results are 0.
    #[must_use]
    pub fn eval(&self, t: f64) -> f32 {
        self.run(t as f32, 0.0, 0.0)
    }

    /// Evaluates a wavetable formula at phase `x` and frame position `y`.
    /// Non-finite results are 0.
    #[must_use]
    pub fn eval_table(&self, x: f32, y: f32) -> f32 {
        self.run(0.0, x, y)
    }

    fn run(&self, t: f32, x: f32, y: f32) -> f32 {
        let mut stack = [0.0_f32; MAX_STACK];
        let mut sp = 0_usize;
        for op in self.ops.iter().take(self.len) {
            let push = |stack: &mut [f32; MAX_STACK], sp: &mut usize, v: f32| {
                if let Some(slot) = stack.get_mut(*sp) {
                    *slot = v;
                    *sp += 1;
                }
            };
            let pop = |stack: &[f32; MAX_STACK], sp: &mut usize| {
                *sp = sp.saturating_sub(1);
                stack.get(*sp).copied().unwrap_or(0.0)
            };
            match *op {
                Op::Num(v) => push(&mut stack, &mut sp, v),
                Op::T => push(&mut stack, &mut sp, t),
                Op::Bar => push(&mut stack, &mut sp, t / 4.0),
                Op::X => push(&mut stack, &mut sp, x),
                Op::Y => push(&mut stack, &mut sp, y),
                Op::Neg => {
                    let a = pop(&stack, &mut sp);
                    push(&mut stack, &mut sp, -a);
                }
                Op::Call(func) if func.arity() == 1 => {
                    let a = pop(&stack, &mut sp);
                    let phase = a.rem_euclid(1.0);
                    let v = match func {
                        Func::Sin => a.sin(),
                        Func::Cos => a.cos(),
                        Func::Abs => a.abs(),
                        Func::Floor => a.floor(),
                        Func::Frac => a.rem_euclid(1.0),
                        Func::Sqrt => a.max(0.0).sqrt(),
                        Func::Saw => phase * 2.0 - 1.0,
                        Func::Tri => 1.0 - 4.0 * (phase - 0.5).abs(),
                        Func::Sqr => {
                            if phase < 0.5 {
                                1.0
                            } else {
                                -1.0
                            }
                        }
                        Func::Min | Func::Max | Func::Pow => 0.0,
                    };
                    push(&mut stack, &mut sp, v);
                }
                binary => {
                    let b = pop(&stack, &mut sp);
                    let a = pop(&stack, &mut sp);
                    let v = match binary {
                        Op::Add => a + b,
                        Op::Sub => a - b,
                        Op::Mul => a * b,
                        Op::Div => a / b,
                        Op::Mod => a.rem_euclid(b),
                        Op::Call(Func::Min) => a.min(b),
                        Op::Call(Func::Max) => a.max(b),
                        Op::Call(Func::Pow) => a.abs().powf(b),
                        _ => 0.0,
                    };
                    push(&mut stack, &mut sp, v);
                }
            }
        }
        let result = sp
            .checked_sub(1)
            .and_then(|i| stack.get(i))
            .copied()
            .unwrap_or(0.0);
        if result.is_finite() { result } else { 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(text: &str, t: f64) -> f32 {
        Formula::parse(text).unwrap().eval(t)
    }

    #[test]
    fn arithmetic_follows_precedence() {
        assert_eq!(eval("1 + 2 * 3", 0.0), 7.0);
        assert_eq!(eval("(1 + 2) * 3", 0.0), 9.0);
        assert_eq!(eval("-2 * -3", 0.0), 6.0);
        assert_eq!(eval("7 % 4 - 1", 0.0), 2.0);
        assert_eq!(eval("10 / 4", 0.0), 2.5);
    }

    #[test]
    fn variables_and_functions() {
        assert_eq!(eval("t", 6.0), 6.0);
        assert_eq!(eval("bar", 6.0), 1.5);
        assert_eq!(eval("frac(t / 4)", 6.0), 0.5);
        assert_eq!(eval("max(0.2, min(t, 0.9))", 3.0), 0.9);
        assert_eq!(eval("pow(2, 3)", 0.0), 8.0);
        assert_eq!(eval("saw(0.75)", 0.0), 0.5);
        assert_eq!(eval("tri(0.5)", 0.0), 1.0);
        assert_eq!(eval("sqr(0.6)", 0.0), -1.0);
        assert!((eval("0.5 + 0.5 * sin(t * pi / 2)", 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn bad_formulas_are_refused_and_bad_results_are_zero() {
        for bad in ["", "1 +", "foo(1)", "(1", "1 2", "min(1)", "sin 1", "1 $ 2"] {
            assert!(Formula::parse(bad).is_err(), "{bad}");
        }
        let long = vec!["1"; 40].join("+");
        assert!(Formula::parse(&long).is_err());
        assert_eq!(eval("1 / 0", 0.0), 0.0);
        assert_eq!(eval("sqrt(-4)", 0.0), 0.0);
        assert_eq!(Formula::default().eval(3.0), 0.0);
    }

    #[test]
    fn table_formulas_use_x_and_y_only() {
        let f = Formula::parse_table("sin(x*pi*2) * (1-y)").unwrap();
        assert!((f.eval_table(0.25, 0.0) - 1.0).abs() < 1e-6);
        assert!((f.eval_table(0.25, 0.5) - 0.5).abs() < 1e-6);
        assert!(Formula::parse_table("t").is_err());
        assert!(Formula::parse_table("bar").is_err());
        assert!(Formula::parse("x").is_err());
        assert!(Formula::parse("y + 1").is_err());
    }
}
