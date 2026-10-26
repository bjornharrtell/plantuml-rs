//! Tokenizer and drawing of one rule of an EBNF diagram (`EbnfExpression`).

use super::ebnf_engine::{syntax_error, EbnfEngine};
use super::note;
use super::shunting_yard::ShuntingYard;
use super::symbol::Symbol;
use super::title_box::TitleBox;
use super::token::Token;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::{with_margin, TextBlock, VerticalBlock};
use crate::skin::SkinParam;
use crate::style::{Margins, Style};
use crate::text::BlocLines;
use std::rc::Rc;

/// One line of a rule, walked character by character (`CharInspectorImpl`).
///
/// The lines are trimmed, and looking past the end of a line gives `'\0'`.
struct CharInspector {
    lines: Vec<Vec<char>>,
    line: usize,
    pos: usize,
    finished: bool,
}

impl CharInspector {
    fn new(lines: &BlocLines) -> Self {
        let lines = lines
            .iter()
            .map(|l| l.trimmed().string().chars().collect::<Vec<char>>())
            .filter(|l| !l.is_empty())
            .collect();
        Self {
            lines,
            line: 0,
            pos: 0,
            finished: false,
        }
    }

    fn peek(&self, ahead: usize) -> char {
        if self.finished {
            return '\0';
        }
        let current = &self.lines[self.line];
        match current.get(self.pos + ahead) {
            Some(c) => *c,
            None => '\0',
        }
    }

    /// Java throws when stepping past the end; the port just stays there.
    fn jump(&mut self) {
        if self.finished {
            return;
        }
        self.pos += 1;
        if self.pos >= self.lines[self.line].len() {
            self.line += 1;
            self.pos = 0;
        }
        if self.line >= self.lines.len() {
            self.finished = true;
        }
    }
}

/// The name of a rule, its tokens and its postfix queue (`EbnfExpression`).
#[derive(Debug, Clone)]
pub struct EbnfExpression {
    tokens: Vec<Token>,
    rpn: Vec<Token>,
    comment_above: Option<String>,
    comment_below: Option<String>,
}

impl EbnfExpression {
    /// Tokenizes a rule and converts it to postfix order (`EbnfExpression.create`).
    ///
    /// Java's `isCompact` argument is stored there but never read, so the port drops it.
    pub fn create(
        lines: &BlocLines,
        comment_above: Option<String>,
        comment_below: Option<String>,
    ) -> Result<Self, String> {
        let tokens = tokenize(lines);
        if tokens.is_empty() {
            return Err("Unparsable expression".to_string());
        }
        if tokens.len() < 2 || tokens[1].symbol() != Symbol::Definition {
            return Err("Syntax error: expected 'name = expression ;'".to_string());
        }
        let rest = &tokens[2..];
        let yard = ShuntingYard::new(rest.iter().cloned())?;
        let rpn = yard.output_queue().to_vec();
        if rpn.is_empty() {
            if !rest.is_empty() {
                return Err("Syntax error: empty expression".to_string());
            }
        } else {
            EbnfEngine::validate(&rpn)?;
        }
        Ok(Self {
            tokens,
            rpn,
            comment_above,
            comment_below,
        })
    }

    /// The name on the left of the `=`.
    pub fn name(&self) -> &str {
        self.tokens.first().map_or("", Token::text)
    }

    /// The tokens of the whole rule, name and `=` included; used by the tests.
    #[allow(dead_code)]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// The postfix queue of the right-hand side; used by the tests.
    #[allow(dead_code)]
    pub fn rpn(&self) -> &[Token] {
        &self.rpn
    }

    /// Title, comments and syntax diagram of the rule (`getUDrawable`).
    pub fn text_block(&self, skin: &Rc<SkinParam>) -> Rc<dyn TextBlock> {
        let style = super::style(skin);
        let fc = style.font_configuration(None);
        if self.tokens.is_empty() {
            return syntax_error(skin, &fc);
        }

        let mut title_box: Rc<dyn TextBlock> = Rc::new(TitleBox::new(self.name(), &fc));
        if let Some(above) = &self.comment_above {
            title_box = Rc::new(VerticalBlock {
                blocks: vec![
                    note::create(skin, &fc, above, Margins::new(0.0, 0.0, 0.0, 10.0)),
                    title_box,
                ],
                alignment: HorizontalAlignment::Center,
            });
        }
        if let Some(below) = &self.comment_below {
            title_box = Rc::new(VerticalBlock {
                blocks: vec![
                    title_box,
                    note::create(skin, &fc, below, Margins::new(10.0, 0.0, 0.0, 0.0)),
                ],
                alignment: HorizontalAlignment::Center,
            });
        }

        let main = self.main_drawing(skin, &style, &fc);
        Rc::new(VerticalBlock {
            blocks: vec![
                title_box,
                with_margin(main, Margins::new(10.0, 0.0, 15.0, 0.0)),
            ],
            alignment: HorizontalAlignment::Left,
        })
    }

    fn main_drawing(
        &self,
        skin: &Rc<SkinParam>,
        style: &Style,
        fc: &FontConfiguration,
    ) -> Rc<dyn TextBlock> {
        if self.rpn.is_empty() {
            return EbnfEngine::empty_text_block(style);
        }
        match self.build(skin, style, fc) {
            Ok(block) => block,
            Err(message) => {
                log::warn!("EBNF: {message}");
                syntax_error(skin, fc)
            }
        }
    }

    /// Runs the postfix queue through the engine (`getMainDrawing`).
    fn build(
        &self,
        skin: &Rc<SkinParam>,
        style: &Style,
        fc: &FontConfiguration,
    ) -> Result<Rc<dyn TextBlock>, String> {
        let mut engine = EbnfEngine::new(skin.clone(), style.clone(), fc.clone());
        for element in &self.rpn {
            let symbol = element.symbol();
            if symbol.is_text() {
                engine.push(element);
            } else if symbol == Symbol::CommentAbove {
                engine.comment_above(element.text().to_string())?;
            } else if symbol == Symbol::CommentBelow {
                engine.comment_below(element.text().to_string())?;
            } else if symbol == Symbol::Alternation {
                engine.alternation()?;
            } else if symbol == Symbol::Concatenation {
                engine.concatenation()?;
            } else if symbol == Symbol::Optional {
                engine.optional()?;
            } else if symbol == Symbol::RepetitionZeroOrMore {
                engine.repetition_zero_or_more()?;
            } else if symbol == Symbol::RepetitionOneOrMore {
                engine.repetition_one_or_more()?;
            } else if symbol == Symbol::RepetitionSymbol {
                engine.repetition_symbol()?;
            } else {
                return Err(format!("unsupported element {element}"));
            }
        }
        engine.text_block()
    }
}

/// Splits the text of a rule into tokens (`EbnfExpression` constructor).
fn tokenize(lines: &BlocLines) -> Vec<Token> {
    let mut it = CharInspector::new(lines);
    let mut tokens: Vec<Token> = Vec::new();
    loop {
        let ch = it.peek(0);
        if ch.is_whitespace() {
            // skipped
        } else if ch == '-' {
            tokens.push(Token::new(Symbol::Not));
        } else if is_letter_or_digit(ch) {
            tokens.push(Token::with_data(Symbol::Literal, read_litteral(&mut it)));
            continue;
        } else if ch == '*' {
            tokens.push(Token::new(Symbol::RepetitionSymbol));
        } else if ch == '(' && it.peek(1) == '*' {
            let comment = read_comment(&mut it);
            if !comment.trim().is_empty() {
                tokens.push(Token::with_data(Symbol::CommentToken, comment));
            }
            continue;
        } else if ch == ',' {
            tokens.push(Token::new(Symbol::Concatenation));
        } else if ch == '|' {
            tokens.push(Token::new(Symbol::Alternation));
        } else if ch == '=' {
            tokens.push(Token::new(Symbol::Definition));
        } else if ch == '(' {
            tokens.push(Token::new(Symbol::GroupingOpen));
        } else if ch == ')' {
            tokens.push(Token::new(Symbol::GroupingClose));
        } else if ch == '[' {
            tokens.push(Token::new(Symbol::OptionalOpen));
        } else if ch == ']' {
            tokens.push(Token::new(Symbol::OptionalClose));
        } else if ch == '{' {
            tokens.push(Token::new(Symbol::RepetitionOpen));
        } else if ch == '}' && it.peek(1) == '-' {
            tokens.push(Token::new(Symbol::RepetitionMinusClose));
            it.jump();
        } else if ch == '}' {
            tokens.push(Token::new(Symbol::RepetitionClose));
        } else if ch == ';' || ch == '\0' {
            break;
        } else if ch == '"' {
            let literal = read_string(&mut it);
            tokens.push(Token::with_data(Symbol::TerminalString1, protect(&literal)));
        } else if ch == '\'' {
            let literal = read_string(&mut it);
            tokens.push(Token::with_data(Symbol::TerminalString2, protect(&literal)));
        } else if ch == '?' {
            let literal = read_string(&mut it);
            tokens.push(Token::with_data(Symbol::SpecialSequence, protect(&literal)));
        } else {
            tokens.clear();
            return tokens;
        }
        it.jump();
    }
    tokens
}

/// An empty terminal string is shown as a space, so that its box keeps a size.
fn protect(literal: &str) -> String {
    if literal.is_empty() {
        " ".to_string()
    } else {
        literal.to_string()
    }
}

fn read_string(it: &mut CharInspector) -> String {
    let separator = it.peek(0);
    it.jump();
    let mut sb = String::new();
    loop {
        let ch = it.peek(0);
        // Java loops until the separator is closed; stopping at the end of the input keeps a
        // missing quote from hanging.
        if ch == separator || ch == '\0' {
            return sb;
        }
        sb.push(ch);
        it.jump();
    }
}

fn read_litteral(it: &mut CharInspector) -> String {
    let mut sb = String::new();
    loop {
        let ch = it.peek(0);
        if !is_letter_or_digit(ch) {
            return sb;
        }
        sb.push(ch);
        it.jump();
    }
}

fn read_comment(it: &mut CharInspector) -> String {
    let mut sb = String::new();
    it.jump();
    it.jump();
    loop {
        let ch = it.peek(0);
        if ch == '\0' {
            return sb;
        }
        if ch == '*' && it.peek(1) == ')' {
            it.jump();
            it.jump();
            return sb;
        }
        sb.push(ch);
        it.jump();
    }
}

fn is_letter_or_digit(ch: char) -> bool {
    ch == '-' || ch == '_' || ch.is_alphanumeric()
}
