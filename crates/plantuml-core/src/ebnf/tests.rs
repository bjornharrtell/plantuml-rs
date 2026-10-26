//! Tests of the EBNF module: tokenizer, shunting yard and end-to-end rendering.

use super::ebnf_expression::EbnfExpression;
use super::shunting_yard::ShuntingYard;
use super::symbol::Symbol;
use super::token::Token;
use crate::text::BlocLines;

fn rule(text: &str) -> EbnfExpression {
    EbnfExpression::create(&BlocLines::single_string(text), None, None).expect("parses")
}

fn symbols(expression: &EbnfExpression) -> Vec<Symbol> {
    expression.tokens().iter().map(Token::symbol).collect()
}

fn rpn(text: &str) -> Vec<Symbol> {
    let expression = rule(text);
    expression.rpn().iter().map(Token::symbol).collect()
}

#[test]
fn tokenizes_a_rule() {
    let expression = rule(r#"word = letter , { letter }- | "x" | 'y' | ?special? ;"#);
    assert_eq!(expression.name(), "word");
    assert_eq!(
        symbols(&expression),
        vec![
            Symbol::Literal,
            Symbol::Definition,
            Symbol::Literal,
            Symbol::Concatenation,
            Symbol::RepetitionOpen,
            Symbol::Literal,
            Symbol::RepetitionMinusClose,
            Symbol::Alternation,
            Symbol::TerminalString1,
            Symbol::Alternation,
            Symbol::TerminalString2,
            Symbol::Alternation,
            Symbol::SpecialSequence,
        ]
    );
    assert_eq!(expression.tokens()[2].text(), "letter");
    assert_eq!(expression.tokens()[8].text(), "x");
    assert_eq!(expression.tokens()[10].text(), "y");
    assert_eq!(expression.tokens()[12].text(), "special");
}

#[test]
fn tokenizes_comments_and_empty_strings() {
    let expression = rule("a = b (* note *) ;");
    assert_eq!(
        symbols(&expression),
        vec![
            Symbol::Literal,
            Symbol::Definition,
            Symbol::Literal,
            Symbol::CommentToken,
        ]
    );
    assert_eq!(expression.tokens()[3].text(), " note ");

    let expression = rule(r#"a = "" ;"#);
    assert_eq!(expression.tokens()[2].symbol(), Symbol::TerminalString1);
    assert_eq!(expression.tokens()[2].text(), " ");

    // An identifier may contain a hyphen once it started with a letter.
    let expression = rule("a = one-two ;");
    assert_eq!(expression.tokens()[2].text(), "one-two");
}

#[test]
fn shunting_yard_orders_operators_by_precedence() {
    assert_eq!(
        rpn("a = x , y | z ;"),
        vec![
            Symbol::Literal,
            Symbol::Literal,
            Symbol::Concatenation,
            Symbol::Literal,
            Symbol::Alternation,
        ]
    );
    assert_eq!(
        rpn("a = x | y , z ;"),
        vec![
            Symbol::Literal,
            Symbol::Literal,
            Symbol::Literal,
            Symbol::Concatenation,
            Symbol::Alternation,
        ]
    );
    assert_eq!(rpn("a = [ x ] ;"), vec![Symbol::Literal, Symbol::Optional]);
    assert_eq!(
        rpn("a = { x } ;"),
        vec![Symbol::Literal, Symbol::RepetitionZeroOrMore]
    );
    assert_eq!(
        rpn("a = { x }- ;"),
        vec![Symbol::Literal, Symbol::RepetitionOneOrMore]
    );
    assert_eq!(
        rpn("a = ( x , y ) | z ;"),
        vec![
            Symbol::Literal,
            Symbol::Literal,
            Symbol::Concatenation,
            Symbol::Literal,
            Symbol::Alternation,
        ]
    );
}

#[test]
fn shunting_yard_flushes_comments() {
    // A comment before the token it documents comes out above it, one after comes out below.
    assert_eq!(
        rpn("a = (* c *) x ;"),
        vec![Symbol::Literal, Symbol::CommentAbove]
    );
    assert_eq!(
        rpn("a = x (* c *) ;"),
        vec![Symbol::Literal, Symbol::CommentBelow]
    );
}

#[test]
fn shunting_yard_reports_unbalanced_groups() {
    let after = |tokens: Vec<Token>| ShuntingYard::new(tokens.into_iter()).unwrap_err();
    let literal = || Token::with_data(Symbol::Literal, "x");
    let error = after(vec![
        Token::new(Symbol::GroupingOpen),
        literal(),
        Token::new(Symbol::OptionalClose),
    ]);
    assert!(error.contains("'['"), "{error}");
    assert!(after(vec![Token::new(Symbol::OptionalOpen), literal()]).contains("unclosed"));
    assert!(after(vec![Token::new(Symbol::RepetitionOpen), literal()]).contains("'{'"));
    assert!(after(vec![Token::new(Symbol::RepetitionClose)]).contains("'{'"));
    assert!(after(vec![Token::new(Symbol::Termination)]).contains("unexpected"));
}

#[test]
fn rejects_unparsable_rules() {
    for text in ["a = ! ;", "a 1 ;", "= \"x\" ;"] {
        assert!(
            EbnfExpression::create(&BlocLines::single_string(text), None, None).is_err(),
            "{text}"
        );
    }
    // A rule without right-hand side is valid: it draws two circles face to face.
    let empty = rule("a = ;");
    assert!(empty.rpn().is_empty());
}

#[test]
fn renders_a_rule() {
    let svg = crate::render(
        "@startebnf\ntitle x\nletter = \"a\" | \"b\" ;\nword = { letter } ;\n@endebnf",
    )
    .expect("renders");
    assert!(svg.contains("data-diagram-type=\"EBNF\""));
    for text in ["letter", "word", "a", "b", "x"] {
        assert!(svg.contains(&format!(">{text}</text>")), "{text} in {svg}");
    }
}

#[test]
fn renders_every_construct() {
    let svg = crate::render(
        "@startebnf\n(* a free note *)\nitem = [ name ] , { name }- , 3 * name , ?free text? ;\n\
         empty = ;\n@endebnf",
    )
    .expect("renders");
    for text in [
        "item",
        "name",
        "empty",
        "a free note",
        "free text",
        "3\u{d7}",
    ] {
        assert!(svg.contains(&format!(">{text}</text>")), "{text} in {svg}");
    }
    assert!(svg.contains("<ellipse"), "start and end circles");
}

#[test]
fn renders_a_rule_over_several_lines() {
    let svg =
        crate::render("@startebnf\nword = letter ,\n{ letter } ;\n@endebnf").expect("renders");
    assert!(svg.contains(">word</text>") && svg.contains(">letter</text>"));
}

#[test]
fn renders_a_comment_above_a_rule() {
    let svg = crate::render("@startebnf\n(* above *) a = \"x\" ;\n@endebnf").expect("renders");
    assert!(svg.contains(">above</text>") && svg.contains(">a</text>"));
}

#[test]
fn renders_comments_around_a_token() {
    let svg = crate::render("@startebnf\na = (* above *) \"x\" (* below *) , b ;\n@endebnf")
        .expect("renders");
    assert!(svg.contains(">above</text>") && svg.contains(">below</text>"));
    assert!(svg.contains(">x</text>") && svg.contains(">b</text>"));
}

#[test]
fn malformed_rules_are_syntax_errors() {
    for source in [
        "@startebnf\na = ( \"b\" ;\n@endebnf",
        "@startebnf\na = \"b\" ] ;\n@endebnf",
        "@startebnf\na = , \"b\" ;\n@endebnf",
        "@startebnf\na = \"b\" ! ;\n@endebnf",
        "@startebnf\nnonsense\n@endebnf",
        "@startebnf\n@endebnf",
    ] {
        assert!(
            matches!(crate::render(source), Err(crate::Error::Syntax(_))),
            "{source}"
        );
    }
}

#[test]
fn unsupported_operators_are_reported() {
    let error = EbnfExpression::create(&BlocLines::single_string("a = b - c ;"), None, None)
        .expect_err("the '-' operator is not supported");
    assert!(error.contains("'-'"), "{error}");
}
