//! Builds the tiles of a rule from its postfix token queue (`EbnfEngine`).

use super::etile::{ETile, TileBlock};
use super::etile_alternation::ETileAlternation;
use super::etile_box::ETileBox;
use super::etile_concatenation::ETileConcatenation;
use super::etile_empty::ETileEmpty;
use super::etile_one_or_more::ETileOneOrMore;
use super::etile_optional2::ETileOptional2;
use super::etile_with_circles::ETileWithCircles;
use super::etile_zero_or_more::ETileZeroOrMore;
use super::symbol::Symbol;
use super::token::Token;
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::font::config::FontConfiguration;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::skin::SkinParam;
use crate::style::{PName, Style};
use std::rc::Rc;

/// Stack-based builder of the tile tree of one rule (`EbnfEngine`).
pub struct EbnfEngine {
    stack: Vec<Box<dyn ETile>>,
    skin: Rc<SkinParam>,
    style: Style,
    fc: FontConfiguration,
    line_color: HColor,
}

impl EbnfEngine {
    pub fn new(skin: Rc<SkinParam>, style: Style, fc: FontConfiguration) -> Self {
        let line_color = style.value(PName::LineColor).as_color();
        Self {
            stack: Vec::new(),
            skin,
            style,
            fc,
            line_color,
        }
    }

    pub fn push(&mut self, element: &Token) {
        let tile = ETileBox::new(
            element.text(),
            element.symbol(),
            self.fc.clone(),
            self.style.clone(),
            self.skin.clone(),
        );
        self.stack.push(Box::new(tile));
    }

    pub fn optional(&mut self) -> Result<(), String> {
        let arg = self.pop()?;
        self.stack.push(Box::new(ETileOptional2::new(
            arg,
            self.skin.clone(),
            self.fc.clone(),
        )));
        Ok(())
    }

    pub fn repetition_zero_or_more(&mut self) -> Result<(), String> {
        let arg = self.pop()?;
        self.stack.push(Box::new(ETileZeroOrMore::new(
            arg,
            self.skin.clone(),
            self.fc.clone(),
        )));
        Ok(())
    }

    pub fn repetition_one_or_more(&mut self) -> Result<(), String> {
        let arg = self.pop()?;
        self.stack.push(Box::new(ETileOneOrMore::new(arg)));
        Ok(())
    }

    /// `expression #n`: repeats `expression` `n` times, labelled with the count.
    pub fn repetition_symbol(&mut self) -> Result<(), String> {
        let arg = self.pop()?;
        let label = self.pop()?.repetition_label();
        let loop_label = format!("{label}\u{00D7}");
        let tile = ETileOneOrMore::with_label(arg, &loop_label, self.fc.bigger(-2.0));
        if label == "0" {
            self.stack.push(Box::new(ETileOptional2::new(
                Box::new(tile),
                self.skin.clone(),
                self.fc.clone(),
            )));
        } else {
            self.stack.push(Box::new(tile));
        }
        Ok(())
    }

    pub fn alternation(&mut self) -> Result<(), String> {
        let mut arg1 = self.pop()?;
        let mut arg2 = self.pop()?;
        if arg1.as_alternation().is_some() {
            arg1.as_alternation().expect("checked").push(arg2);
            self.stack.push(arg1);
        } else if arg2.as_alternation().is_some() {
            arg2.as_alternation().expect("checked").push(arg1);
            self.stack.push(arg2);
        } else {
            let mut alternation = ETileAlternation::new();
            alternation.push(arg1);
            alternation.push(arg2);
            self.stack.push(Box::new(alternation));
        }
        Ok(())
    }

    pub fn concatenation(&mut self) -> Result<(), String> {
        let mut arg1 = self.pop()?;
        let mut arg2 = self.pop()?;
        if arg1.as_concatenation().is_some() {
            arg1.as_concatenation().expect("checked").push(arg2);
            self.stack.push(arg1);
        } else if arg2.as_concatenation().is_some() {
            arg2.as_concatenation().expect("checked").push_last(arg1);
            self.stack.push(arg2);
        } else {
            let mut concatenation = ETileConcatenation::new();
            concatenation.push(arg1);
            concatenation.push(arg2);
            self.stack.push(Box::new(concatenation));
        }
        Ok(())
    }

    pub fn comment_above(&mut self, comment: String) -> Result<(), String> {
        let tile = self
            .stack
            .last_mut()
            .ok_or_else(|| "Syntax error: comment without a rule".to_string())?;
        tile.add_comment_above(comment);
        Ok(())
    }

    pub fn comment_below(&mut self, comment: String) -> Result<(), String> {
        let tile = self
            .stack
            .last_mut()
            .ok_or_else(|| "Syntax error: comment without a rule".to_string())?;
        tile.add_comment_below(comment);
        Ok(())
    }

    /// The finished drawing, framed by the two circles (`getTextBlock`).
    pub fn text_block(self) -> Result<Rc<dyn TextBlock>, String> {
        if self.stack.len() != 1 {
            return Err("Syntax error: incomplete expression".to_string());
        }
        let root = self.stack.into_iter().next().expect("one tile");
        let with_circles = ETileWithCircles::new(Rc::from(root), self.line_color);
        Ok(Rc::new(TileBlock::new(Rc::new(with_circles))))
    }

    /// The drawing of a rule without right-hand side: two circles and nothing between them.
    pub fn empty_text_block(style: &Style) -> Rc<dyn TextBlock> {
        let line_color = style.value(PName::LineColor).as_color();
        let empty: Rc<dyn ETile> = Rc::new(ETileEmpty);
        Rc::new(TileBlock::new(Rc::new(ETileWithCircles::new(
            empty, line_color,
        ))))
    }

    fn pop(&mut self) -> Result<Box<dyn ETile>, String> {
        self.stack
            .pop()
            .ok_or_else(|| "Syntax error: missing operand".to_string())
    }

    /// Checks the postfix queue without drawing it, so that errors are reported while parsing.
    pub fn validate(queue: &[Token]) -> Result<(), String> {
        let mut depth: usize = 0;
        for token in queue {
            match token.symbol() {
                symbol if symbol.is_text() => depth += 1,
                Symbol::CommentAbove | Symbol::CommentBelow => {
                    if depth == 0 {
                        return Err("Syntax error: comment without a rule".to_string());
                    }
                }
                Symbol::Alternation | Symbol::Concatenation | Symbol::RepetitionSymbol => {
                    if depth < 2 {
                        return Err(
                            "Syntax error: an operator is missing one of its operands".to_string()
                        );
                    }
                    depth -= 1;
                }
                Symbol::Optional | Symbol::RepetitionZeroOrMore | Symbol::RepetitionOneOrMore => {
                    if depth == 0 {
                        return Err("Syntax error: an operator is missing its operand".to_string());
                    }
                }
                Symbol::Not => {
                    return Err("Syntax error: the '-' operator is not supported yet".to_string());
                }
                other => {
                    return Err(format!(
                        "Syntax error: unsupported element {}",
                        Token::new(other)
                    ));
                }
            }
        }
        if depth != 1 {
            return Err("Syntax error: incomplete expression".to_string());
        }
        Ok(())
    }
}

/// The picture Java shows instead of a drawing it could not build (`EbnfEngine.syntaxError`).
pub fn syntax_error(skin: &SkinParam, fc: &FontConfiguration) -> Rc<dyn TextBlock> {
    Display::create(&["Syntax error!"]).create_simple_block(
        fc,
        HorizontalAlignment::Left,
        &skin.skin_simple(),
    )
}
