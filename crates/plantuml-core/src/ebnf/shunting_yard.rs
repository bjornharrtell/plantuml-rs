//! Shunting yard turning the tokens of a rule into a reverse polish queue (`ShuntingYard`).

use super::symbol::Symbol;
use super::token::Token;
use std::collections::VecDeque;

/// Queue of tokens in postfix order, built with the shunting yard algorithm (`ShuntingYard`).
///
/// The queue is what [`EbnfEngine`](super::ebnf_engine::EbnfEngine) consumes: operands first, then
/// the operator that combines them.
#[derive(Debug, Clone, Default)]
pub struct ShuntingYard {
    output_queue: Vec<Token>,
    /// Top of the stack is the front.
    operator_stack: VecDeque<Token>,
}

impl ShuntingYard {
    /// Converts `tokens` to postfix order; the error message explains what is unbalanced.
    pub fn new(tokens: impl Iterator<Item = Token>) -> Result<Self, String> {
        let mut yard = Self::default();
        for token in tokens {
            yard.process(token)?;
        }
        yard.finish()?;
        Ok(yard)
    }

    pub fn output_queue(&self) -> &[Token] {
        &self.output_queue
    }

    fn process(&mut self, token: Token) -> Result<(), String> {
        let symbol = token.symbol();
        if symbol.is_text() {
            self.output_queue.push(token);
            while self.comment_on_top() {
                let comment = self.remove_first_above();
                self.output_queue.push(comment);
            }
        } else if symbol == Symbol::CommentToken {
            self.operator_stack.push_front(token);
        } else if symbol == Symbol::Not {
            self.operator_stack
                .push_front(Token::new(Symbol::Concatenation));
            self.operator_stack.push_front(token);
        } else if symbol.is_operator() {
            while self.operator_on_top_with_greater_precedence(symbol) || self.comment_on_top() {
                let top = self.remove_first_below();
                self.output_queue.push(top);
            }
            self.operator_stack.push_front(token);
        } else if symbol == Symbol::GroupingOpen {
            self.operator_stack.push_front(token);
        } else if symbol == Symbol::GroupingClose {
            while self
                .operator_stack
                .front()
                .is_some_and(|t| t.symbol() != Symbol::GroupingOpen)
            {
                let top = self.remove_first_below();
                self.output_queue.push(top);
            }
            if self.operator_stack.pop_front().is_none() {
                return Err("Syntax error: missing '('".to_string());
            }
        } else if symbol == Symbol::OptionalOpen {
            self.operator_stack.push_front(Token::new(Symbol::Optional));
        } else if symbol == Symbol::OptionalClose {
            while self.operator_on_top() || self.comment_on_top() {
                let top = self.remove_first_below();
                self.output_queue.push(top);
            }
            match self.operator_stack.pop_front() {
                Some(first) if first.symbol() == Symbol::Optional => self.output_queue.push(first),
                _ => return Err("Syntax error: matching '[' not found".to_string()),
            }
            while self.comment_on_top() {
                let comment = self.remove_first_above();
                self.output_queue.push(comment);
            }
        } else if symbol == Symbol::RepetitionOpen {
            self.operator_stack
                .push_front(Token::new(Symbol::RepetitionZeroOrMore));
        } else if symbol == Symbol::RepetitionClose || symbol == Symbol::RepetitionMinusClose {
            while self.operator_on_top() || self.comment_on_top() {
                let top = self.remove_first_below();
                self.output_queue.push(top);
            }
            match self.operator_stack.pop_front() {
                Some(first) if first.symbol() == Symbol::RepetitionZeroOrMore => {
                    let result = if symbol == Symbol::RepetitionMinusClose {
                        Symbol::RepetitionOneOrMore
                    } else {
                        Symbol::RepetitionZeroOrMore
                    };
                    self.output_queue.push(Token::new(result));
                }
                _ => return Err("Syntax error: matching '{' not found".to_string()),
            }
            while self.comment_on_top() {
                let comment = self.remove_first_above();
                self.output_queue.push(comment);
            }
        } else {
            return Err(format!("Syntax error: unexpected token {token}"));
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        while let Some(token) = self.operator_stack.pop_front() {
            if matches!(
                token.symbol(),
                Symbol::Optional | Symbol::RepetitionOneOrMore | Symbol::RepetitionZeroOrMore
            ) {
                let name = match token.symbol() {
                    Symbol::Optional => "[",
                    _ => "{",
                };
                self.output_queue.clear();
                return Err(format!("Syntax error: unclosed '{name}'"));
            }
            if token.symbol() == Symbol::CommentToken {
                self.output_queue
                    .push(Token::with_data(Symbol::CommentBelow, token.text()));
            } else {
                self.output_queue.push(token);
            }
        }
        Ok(())
    }

    /// Pops the top of the stack, marking a comment as following its token (`removeFirstAbove`).
    fn remove_first_above(&mut self) -> Token {
        let result = self.operator_stack.pop_front().expect("not empty");
        if result.symbol() == Symbol::CommentToken {
            return Token::with_data(Symbol::CommentAbove, result.text());
        }
        result
    }

    /// Pops the top of the stack, marking a comment as preceding its token (`removeFirstBelow`).
    fn remove_first_below(&mut self) -> Token {
        let result = self.operator_stack.pop_front().expect("not empty");
        if result.symbol() == Symbol::CommentToken {
            return Token::with_data(Symbol::CommentBelow, result.text());
        }
        result
    }

    fn comment_on_top(&self) -> bool {
        self.operator_stack
            .front()
            .is_some_and(|t| t.symbol() == Symbol::CommentToken)
    }

    fn operator_on_top(&self) -> bool {
        self.operator_stack
            .front()
            .is_some_and(|t| t.symbol().is_operator())
    }

    fn operator_on_top_with_greater_precedence(&self, token: Symbol) -> bool {
        self.operator_stack
            .front()
            .is_some_and(|t| t.symbol().is_operator() && t.symbol().priority() > token.priority())
    }
}
