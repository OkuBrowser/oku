//! Lexer
//! 
//! - Input: `String`
//! - Output: `Vec<Token>`
//! 

use super::{
    token::Token, 
    attribute::Variant,
};

/// Lexer
#[derive(Debug, Clone)]
pub(crate) struct Lexer<'a> {
    input: std::str::Chars<'a>,
    pub(crate) cur: char,
    pub(crate) peek: char,
}

impl<'a> Lexer<'a> {
    /// 入力ソースコードを受け取り Lexer インスタンスを生成する.
    pub(crate) fn new(input: &'a str) -> Self {
        let mut lexer = Lexer { 
            input: input.chars(),
            cur:  '\u{0}',
            peek: '\u{0}',
        };
        lexer.read_char();
        lexer.read_char();
        lexer
    }

    /// 1 文字進む.
    pub(crate) fn read_char(&mut self) -> char {
        let c = self.cur;
        self.cur = self.peek;
        self.peek = self.input.next().unwrap_or('\u{0}');
        c
    }

    /// 空白文字をスキップする.
    fn skip_whitespace(&mut self) {
        while self.cur == ' ' || self.cur == '\t' || self.cur == '\n' || self.cur == '\r' {
            self.read_char();
        }
    }

    /// コマンド一つ分を読み込みトークンに変換する.
    fn read_command(&mut self) -> Token {
        // `\\` を読み飛ばす
        self.read_char();
        let mut command = String::new();
        // 1 文字は確実に読む
        let first = self.read_char();
        command.push(first);
        // ASCII アルファベットなら続けて読む
        while first.is_ascii_alphabetic() && self.cur.is_ascii_alphabetic() {
            command.push(self.read_char());
        }

        Token::from_command(&command)
    }
    
    /// 数字一つ分を読み込みトークンに変換する.
    fn read_number(&mut self) -> Token {
        let mut number = String::new();
        let mut has_period = false;
        while self.cur.is_ascii_digit() || (self.cur == '.' && !has_period) {
            if self.cur == '.' { has_period = true; }
            number.push(self.read_char());
        }
        Token::Number(number)
    }

    /// 次のトークンを生成する.
    pub(crate) fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let token = match self.cur {
            '=' => Token::Operator('='),
            ';' => Token::Operator(';'),
            ',' => Token::Operator(','),
            '.' => Token::Operator('.'),
            '\'' => Token::Operator('\''),
            '(' => Token::Paren("("),
            ')' => Token::Paren(")"),
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::Paren("["),
            ']' => Token::Paren("]"),
            '|' => Token::Paren("|"),
            '+' => Token::Operator('+'), 
            '-' => Token::Operator('-'),
            '*' => Token::Operator('*'),
            '/' => Token::Operator('/'),
            '!' => Token::Operator('!'),
            '<' => Token::Operator('<'), 
            '>' => Token::Operator('>'), 
            '_' => Token::Underscore,
            '^' => Token::Circumflex,
            '&' => Token::Ampersand,
            '\u{0}' => Token::EOF,
            ':' => if self.peek == '=' {
                self.read_char();
                Token::Paren(":=")
            } else { Token::Operator(':') },
            '\\' => { return self.read_command(); },
            c => {
                if c.is_ascii_digit() {
                    return self.read_number();
                } else if c.is_ascii_alphabetic() {
                    Token::Letter(c, Variant::Italic)
                } else {
                    Token::Letter(c, Variant::Normal)
                }
            },
        };
        self.read_char();
        token
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        token::Token, 
        attribute::Variant
    };
    use super::*;

    #[test]
    fn lexer_test() {
        let problems = vec![
            (r"3", vec![Token::Number("3".to_owned())]),
            (r"3.14", vec![Token::Number("3.14".to_owned())]),
            (r"3.14.", vec![Token::Number("3.14".to_owned()), Token::Operator('.')]),
            (r"x", vec![Token::Letter('x', Variant::Italic)]),
            (r"\pi", vec![Token::Letter('π', Variant::Italic)]),
            (r"x = 3.14", vec![
                Token::Letter('x', Variant::Italic), 
                Token::Operator('='), 
                Token::Number("3.14".to_owned())
            ]),
            (r"\alpha\beta", vec![Token::Letter('α', Variant::Italic), Token::Letter('β', Variant::Italic)]),
            (r"x+y", vec![Token::Letter('x', Variant::Italic), Token::Operator('+'), Token::Letter('y', Variant::Italic)]),
            (r"\ 1", vec![Token::Space(1.), Token::Number("1".to_owned())]),
        ];

        for (problem, answer) in problems.iter() {
            let mut lexer = Lexer::new(problem);
            for answer in answer.iter() {
                assert_eq!(&lexer.next_token(), answer);
            }
        }
    }
}