use super::{
    attribute::{Variant, Accent, LineThickness, ColumnAlign},
    token::Token, 
    lexer::Lexer,
    ast::Node,
    error::LatexError,
};

#[derive(Debug, Clone)]
pub(crate) struct Parser<'a> {
    l: Lexer<'a>,
    cur_token: Token,
    peek_token: Token,
}
impl<'a> Parser<'a> {
    pub(crate) fn new(l: Lexer<'a>) -> Self {
        let mut p = Parser { 
            l, 
            cur_token: Token::Illegal('\u{0}'),
            peek_token: Token::Illegal('\u{0}'),
        };
        p.next_token();
        p.next_token();
        p
    }

    fn next_token(&mut self) {
        self.cur_token = self.peek_token.clone();
        self.peek_token = if 
            self.cur_token.acts_on_a_digit() && self.l.cur.is_ascii_digit() 
        {
            let num = self.l.cur;
            self.l.read_char();
            Token::Number(format!("{}", num))
        } else {
            self.l.next_token()
        };
    }

    fn cur_token_is(&self, expected_token: &Token) -> bool {
        &self.cur_token == expected_token
    }

    fn peek_token_is(&self, expected_token: Token) -> bool {
        self.peek_token == expected_token
    }

    pub(crate) fn parse(&mut self) -> Result<Vec<Node>, LatexError> {
        let mut nodes = Vec::new();

        while !self.cur_token_is(&Token::EOF) {
            nodes.push(
                self.parse_node()?
            );
            self.next_token();
        }

        Ok(nodes)
    }

    fn parse_node(&mut self) -> Result<Node, LatexError> {
        let left = self.parse_single_node()?;

        match self.peek_token {
            Token::Underscore => {
                self.next_token();
                self.next_token();
                let right = self.parse_node()?;
                Ok(Node::Subscript(Box::new(left), Box::new(right)))
            }
            Token::Circumflex => {
                self.next_token();
                self.next_token();
                let right = self.parse_node()?;
                Ok(Node::Superscript(Box::new(left), Box::new(right)))
            },
            _ => Ok(left),
        }
    }

    // 中置演算子 `_`, `^`, '\'' が続くかどうかを気にせずに, 直後のノードを読む
    // 
    // 注) 中置演算子を考慮して正しくノードを読む場合は `parse_node()` を使う.
    fn parse_single_node(&mut self) -> Result<Node, LatexError> {
        let node = match &self.cur_token {
            Token::Number(number) => Node::Number(number.clone()),
            Token::Letter(x, v)   => Node::Letter(*x, *v),
            Token::Operator(op) => Node::Operator(*op),
            Token::Function(fun)  => Node::Function(fun.to_string(), None),
            Token::Space(space) => Node::Space(*space),
            Token::Sqrt => {
                self.next_token();
                let degree = if self.cur_token_is(&Token::Paren("[")) {
                    let degree = self.parse_group(&Token::Paren("]"))?;
                    self.next_token();
                    Some(Box::new(degree))
                } else { None };
                let content = self.parse_node()?;
                Node::Sqrt(degree, Box::new(content))
            },
            Token::Frac => {
                self.next_token();
                let numerator = self.parse_node()?;
                self.next_token();
                let denominator = self.parse_node()?;
                Node::Frac(Box::new(numerator), Box::new(denominator), LineThickness::Medium)
            },
            Token::Binom(display) => {
                let display = *display;
                self.next_token();
                let numerator = self.parse_node()?;
                self.next_token();
                let denominator = self.parse_node()?;

                let binom = Node::Fenced {
                    open: "(",
                    close: ")",
                    content: Box::new(
                        Node::Frac(Box::new(numerator), Box::new(denominator), LineThickness::Length(0))
                    ),
                };
                match display {
                    Some(display) => Node::Style(Some(display), Box::new(Node::Row(vec![binom]))),
                    None          => binom
                }
            },
            Token::Over(op, acc) => {
                let (op, acc) = (*op, *acc);
                self.next_token();
                let target = self.parse_node()?;
                Node::OverOp(op, acc, Box::new(target))
            },
            Token::Under(op, acc) => {
                let (op, acc) = (*op, *acc);
                self.next_token();
                let target = self.parse_node()?;
                Node::UnderOp(op, acc, Box::new(target))
            },
            Token::Overset => {
                self.next_token();
                let over = self.parse_node()?;
                self.next_token();
                let target = self.parse_node()?;
                Node::Overset{over: Box::new(over), target: Box::new(target)}
            },
            Token::Underset => {
                self.next_token();
                let under = self.parse_node()?;
                self.next_token();
                let target = self.parse_node()?;
                Node::Underset{under: Box::new(under), target: Box::new(target)}
            },
            Token::Overbrace(x) => {
                let x = *x;
                self.next_token();
                let target = self.parse_single_node()?;
                if self.peek_token_is(Token::Circumflex) {
                    self.next_token();
                    self.next_token();
                    let expl = self.parse_single_node()?;
                    let over = Node::Overset{over: Box::new(expl), target: Box::new(Node::Operator(x))};
                    Node::Overset{over: Box::new(over), target: Box::new(target)}
                } else {
                    Node::Overset{over: Box::new(Node::Operator(x)), target: Box::new(target)}
                }
            },
            Token::Underbrace(x) => {
                let x = *x;
                self.next_token();
                let target = self.parse_single_node()?;
                if self.peek_token_is(Token::Underscore) {
                    self.next_token();
                    self.next_token();
                    let expl = self.parse_single_node()?;
                    let under = Node::Underset{under: Box::new(expl), target: Box::new(Node::Operator(x))};
                    Node::Underset{under: Box::new(under), target: Box::new(target)}
                } else {
                    Node::Underset{under: Box::new(Node::Operator(x)), target: Box::new(target)}
                }
            },
            Token::BigOp(op) => {
                let op = *op;
                match self.peek_token {
                    Token::Underscore => {
                        self.next_token();
                        self.next_token();
                        let under = self.parse_single_node()?;
                        if self.peek_token_is(Token::Circumflex) {
                            self.next_token();
                            self.next_token();
                            let over = self.parse_single_node()?;
                            Node::UnderOver{ target: Box::new(Node::Operator(op)), under: Box::new(under), over: Box::new(over) }
                        } else {
                            Node::Under(Box::new(Node::Operator(op)), Box::new(under))
                        }
                    },
                    Token::Circumflex => {
                        self.next_token();
                        self.next_token();
                        let over = self.parse_single_node()?;
                        if self.peek_token_is(Token::Underscore) {
                            self.next_token();
                            self.next_token();
                            let under = self.parse_single_node()?;
                            Node::UnderOver{ target: Box::new(Node::Operator(op)), under: Box::new(under), over: Box::new(over) }
                        } else {
                            Node::OverOp(op, Accent::False, Box::new(over))
                        }
                    },
                    _ => Node::Operator(op)
                }
            },
            Token::Lim(lim) => {
                let lim = Node::Function(lim.to_string(), None);
                if self.peek_token_is(Token::Underscore) {
                    self.next_token();
                    self.next_token();
                    let under = self.parse_single_node()?;
                    Node::Under(Box::new(lim), Box::new(under))
                } else {
                    lim
                }
            },
            Token::Slashed => {
                self.next_token();
                self.next_token();
                let node = self.parse_node()?;
                self.next_token();
                Node::Slashed(Box::new(node))
            },
            Token::Style(var) => {
                let var = *var;
                self.next_token();
                let node = self.parse_node()?;
                set_variant(node, var)
            },
            Token::Integral(int) => {
                let int = *int;
                match self.peek_token {
                    Token::Underscore => {
                        self.next_token();
                        self.next_token();
                        let sub = self.parse_single_node()?;
                        if self.peek_token_is(Token::Circumflex) {
                            self.next_token();
                            self.next_token();
                            let sup = self.parse_single_node()?;
                            Node::SubSup{ target: Box::new(Node::Operator(int)), sub: Box::new(sub), sup: Box::new(sup) }
                        } else {
                            Node::Subscript(Box::new(Node::Operator(int)), Box::new(sub))
                        }
                    },
                    Token::Circumflex => {
                        self.next_token();
                        self.next_token();
                        let sup = self.parse_single_node()?;
                        if self.peek_token_is(Token::Underscore) {
                            self.next_token();
                            self.next_token();
                            let sub = self.parse_single_node()?;
                            Node::SubSup{ target: Box::new(Node::Operator(int)), sub: Box::new(sub), sup: Box::new(sup) }
                        } else {
                            Node::Superscript(Box::new(Node::Operator(int)), Box::new(sup))
                        }
                    },
                    _ => Node::Operator(int)
                }
            },
            Token::LBrace => self.parse_group(&Token::RBrace)?,
            Token::Paren(paren) => Node::OtherOperator(paren),
            Token::Left => {
                self.next_token();
                let open = match &self.cur_token {
                    Token::Paren(open) => *open,
                    Token::Operator('.') => "",
                    token => {return Err(LatexError::MissingParensethis{
                        location: Token::Left, got: token.clone(),
                    })},
                };
                let content = self.parse_group(&Token::Right)?;
                self.next_token();
                let close = match &self.cur_token {
                    Token::Paren(close) => close,
                    Token::Operator('.') => "",
                    token => {return Err(LatexError::MissingParensethis{
                        location: Token::Right, got: token.clone(),
                    })},
                };
                Node::Fenced{open, close, content: Box::new(content)}
            },
            Token::Middle => {
                let stretchy = true;
                self.next_token();
                match self.parse_single_node()? {
                    Node::Operator(op) => Node::StrechedOp(stretchy, op.to_string()),
                    Node::OtherOperator(op) => Node::StrechedOp(stretchy, op.to_owned()),
                    _ => unimplemented!()
                }
            },
            Token::Big(size) => {
                let size = *size;
                self.next_token();
                match self.cur_token {
                    Token::Paren(paren) => Node::SizedParen{ size, paren },
                    _ => {return Err(LatexError::UnexpectedToken{
                        expected: Token::Paren(""), got: self.cur_token.clone(),
                    });},
                }
            },
            Token::Begin => {
                self.next_token();
                // 環境名を読み込む
                let environment = self.parse_text();
                let (columnalign, environment) = if &environment == "align" { 
                    (ColumnAlign::Left, "matrix".to_owned())
                } else { (ColumnAlign::Center, environment) };
                // \begin..\end の中身を読み込む
                let content = match self.parse_group(&Token::End)? {
                    Node::Row(content) => content,
                    content => vec![content],
                };
                let content = Node::Matrix(content, columnalign);

                // 環境名により処理を分岐
                let matrix = match environment.as_str() {
                    "matrix"  => content,
                    "pmatrix" => Node::Fenced{open: "(", close: ")", content: Box::new(content)}, 
                    "bmatrix" => Node::Fenced{open: "[", close: "]", content: Box::new(content)}, 
                    "vmatrix" => Node::Fenced{open: "|", close: "|", content: Box::new(content)}, 
                    environment => { return Err(LatexError::UnknownEnvironment(environment.to_owned())); },
                };
                self.next_token();
                let _ = self.parse_text();
                    
                matrix
            },
            Token::OperatorName => {
                self.next_token();
                // 関数名を読み込む
                let function = self.parse_text();
                Node::Function(function, None)
            },
            Token::Text => {
                self.next_token();
                // テキストを読み込む
                let text = self.parse_text();
                Node::Text(text)
            },
            Token::Ampersand => Node::Ampersand,
            Token::NewLine => Node::NewLine,
            token => Node::Undefined(format!("{:?}", token)),
        };

        match self.peek_token {
            Token::Operator('\'') => {
                self.next_token();
                Ok(Node::Superscript(Box::new(node), Box::new(Node::Operator('′'))))
            },
            _ => Ok(node),
        }
    }

    fn parse_group(&mut self, end_token: &Token) -> Result<Node, LatexError> {
        self.next_token();
        let mut nodes = Vec::new();

        while {
            if self.cur_token_is(&Token::EOF) { // 閉じ括弧がないまま入力が終了した場合
                return Err(LatexError::UnexpectedToken{
                    expected: end_token.clone(),
                    got: self.cur_token.clone()
                });
            }

            !self.cur_token_is(end_token) 
        } {
            nodes.push(
                self.parse_node()?
            );
            self.next_token();
        }

        if nodes.len() == 1 {
            let node = nodes.into_iter().nth(0).unwrap();
            Ok(node)
        } else {
            Ok(Node::Row(nodes))
        }
    }

    fn parse_text(&mut self) -> String {
        // `{` を読み飛ばす
        self.next_token();

        // テキストを読み取る
        let mut text = String::new();
        while let Token::Letter(x, _) = self.cur_token {
            text.push(x);
            self.next_token();
        }
        // 終わったら最後の `}` を cur が指した状態で抜ける

        text
    }
}

fn set_variant(node: Node, var: Variant) -> Node {
    match node {
        Node::Letter(x, _) => Node::Letter(x, var),
        Node::Row(vec) => Node::Row(
            vec.into_iter()
                .map(|node| set_variant(node, var))
                .collect()
        ),
        node => node,
    }
}
