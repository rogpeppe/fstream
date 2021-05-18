use logos::Logos;

pub fn parse(s: &str) -> Result<ASTNode> {
    let mut lex = Lexer::new(s);
    parse_pipeline(&mut lex)
}

type Result<T> = std::result::Result<T, String>;

fn parse_pipeline(lex: &mut Lexer) -> Result<ASTNode> {
    println!("parse_pipeline {{");
    let mut node = ASTNode::Command(parse_command(lex)?);
    loop {
        println!("in pipeline loop {:?}", lex.peek());
        match lex.peek() {
            Some(Token::Pipe) => {
                lex.next();
                node = ASTNode::Pipe(Box::new(node), parse_command(lex)?);
            }
            Some(Token::CloseCurly) | None => {
                println!("}}");
                return Ok(node);
            }
            tok => {
                return Err(format!("expected pipe, got {:?}", tok).to_string());
            }
        }
    }
}

fn parse_command(lex: &mut Lexer) -> Result<Command> {
    println!("parse_command {{");
    let name = match lex.next() {
        Some(Token::Word) => lex.string(),
        Some(Token::QuotedWord) => lex.string(),
        tok => {
            return Err(format!("expected word, got {:?}", tok).to_string());
        }
    };
    let mut args = vec![];
    loop {
        println!("command loop: {:?}", lex.peek());
        match lex.peek() {
            Some(Token::Word) => {
                args.push(ASTNode::Word(lex.source[lex.lexer.span()].to_string()));
            }
            Some(Token::QuotedWord) => {
                args.push(ASTNode::Word(unquote(&lex.source[lex.lexer.span()])));
            }
            Some(Token::OpenCurly) => {
                lex.next();
                args.push(parse_pipeline(lex)?);
                match lex.peek() {
                    Some(Token::CloseCurly) => (),
                    _tok => return Err("unexpected token".to_string()), // TODO more info about tok
                }
            }
            None | Some(Token::Pipe) | Some(Token::CloseCurly) => {
                println!("}} XXXX {:?}", lex.peek());
                return Ok(Command {
                    name: name,
                    args: args,
                });
            }
            _ => {
                return Err("unexpected token".to_string());
            }
        }
        lex.next();
    }
}

fn unquote(s: &str) -> String {
    return s[1..s.len() - 1].replace("''", "'");
}

#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub args: Vec<ASTNode>,
}

#[derive(Debug, Clone)]
pub enum ASTNode {
    Command(Command),
    Pipe(Box<ASTNode>, Command),
    Word(String),
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", quote(&self.name))?;
        let args: &Vec<ASTNode> = &self.args; // TODO there must be a neater way of doing this.
        for arg in args {
            if let ASTNode::Command(_) = arg {
                write!(f, " {{{}}}", arg)?;
            } else {
                write!(f, " {}", arg)?;
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for ASTNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(match self {
            ASTNode::Command(c) => {
                write!(f, "{}", c)?;
            }
            ASTNode::Pipe(node, c) => {
                write!(f, "{} | {}", node, c)?;
            }
            ASTNode::Word(s) => {
                write!(f, "{}", quote(&s))?;
            }
        })
    }
}

fn quote(s: &str) -> String {
    if !s.contains('\'') {
        s.to_string()
    } else {
        s.replace("'", "''")
    }
}

struct Lexer<'src> {
    source: &'src str,
    lexer: logos::Lexer<'src, Token>,
    peeked: Option<Option<Token>>,
}

impl<'source> Lexer<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            lexer: Token::lexer(source),
            peeked: None,
            source: source,
        }
    }

    fn peek(&mut self) -> &Option<Token> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next());
        }
        self.peeked.as_ref().unwrap()
    }

    fn str(&self) -> &str {
        &self.source[self.lexer.span()]
    }

    fn string(&self) -> String {
        self.str().to_string()
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        dbg!(if let Some(peeked) = self.peeked.take() {
            peeked
        } else {
            self.lexer.next()
        })
    }
}
// TODO flags with values: -foo=bar, -foo='bar baz'

#[derive(Logos, Debug, PartialEq, Clone)]
enum Token {
    #[regex("[ \r\t]", logos::skip)]
    #[error]
    Error,

    // Tokens can be literal strings, of any length.
    #[token("|")]
    Pipe,

    #[token("{")]
    OpenCurly,

    #[token("}")]
    CloseCurly,

    //#[regex("-[a-zA-Z]+")]
    //Option,
    #[regex("[a-zA-Z0-9/]+")] // TODO allow more chars here
    Word,

    #[regex("'([^']|'')*'")]
    QuotedWord,
}
