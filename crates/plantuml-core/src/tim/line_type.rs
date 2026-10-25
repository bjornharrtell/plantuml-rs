//! Classification of preprocessor lines (`TLineType`).

use crate::regex::IRegex;
use crate::text::StringLocated;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TLineType {
    Plain,
    AffectationDefine,
    Affectation,
    Assert,
    If,
    Ifdef,
    Undef,
    Ifndef,
    Else,
    ElseIf,
    EndIf,
    While,
    EndWhile,
    Foreach,
    EndForeach,
    DeclareReturnFunction,
    DeclareProcedure,
    EndFunction,
    Return,
    LegacyDefine,
    LegacyDefineLong,
    Theme,
    Include,
    IncludeDef,
    Import,
    StartSub,
    EndSub,
    IncludeSub,
    Log,
    DumpMemory,
    CommentSimple,
    CommentLongStart,
    Option,
}

const IDENT: &str = r"[\p{L}_][\p{L}_0-9]*";

fn leaf(s: &str) -> IRegex {
    IRegex::leaf(s)
}

fn keyword(word: &str) -> IRegex {
    IRegex::group(vec![
        IRegex::start(),
        IRegex::space_zero_or_more(),
        leaf(word),
        leaf(r"\b"),
    ])
}

fn concat(parts: Vec<IRegex>) -> IRegex {
    IRegex::group(parts)
}

struct Patterns {
    legacy_define: IRegex,
    legacy_definelong: IRegex,
    affectation_define: IRegex,
    affectation: IRegex,
    comment_simple1: IRegex,
    comment_simple2: IRegex,
    comment_long_start: IRegex,
    ifdef: IRegex,
    undef: IRegex,
    ifndef: IRegex,
    assert: IRegex,
    if_: IRegex,
    declare_return_function: IRegex,
    declare_procedure: IRegex,
    else_: IRegex,
    elseif: IRegex,
    endif: IRegex,
    while_: IRegex,
    endwhile: IRegex,
    foreach: IRegex,
    endfor: IRegex,
    end_function: IRegex,
    return_: IRegex,
    theme: IRegex,
    include: IRegex,
    include_def: IRegex,
    import: IRegex,
    startsub: IRegex,
    endsub: IRegex,
    includesub: IRegex,
    log: IRegex,
    dump_memory: IRegex,
    option: IRegex,
}

fn build_patterns() -> Patterns {
    Patterns {
        legacy_define: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("!define"),
            IRegex::space_one_or_more(),
            leaf(IDENT),
            leaf(r"\("),
        ]),
        legacy_definelong: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("!definelong"),
            IRegex::space_one_or_more(),
            leaf(IDENT),
            leaf(r"\b"),
        ]),
        affectation_define: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("!define"),
            IRegex::space_one_or_more(),
            leaf(IDENT),
            leaf(r"\b"),
        ]),
        affectation: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("!"),
            IRegex::space_zero_or_more(),
            leaf("(local|global)?"),
            IRegex::space_zero_or_more(),
            leaf(r"\$?"),
            leaf(IDENT),
            IRegex::space_zero_or_more(),
            leaf(r"\??="),
        ]),
        comment_simple1: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("'"),
        ]),
        comment_simple2: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("/'"),
            leaf(".*"),
            leaf("'/"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ]),
        comment_long_start: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("/'"),
        ]),
        ifdef: keyword("!ifdef"),
        undef: keyword("!undef"),
        ifndef: keyword("!ifndef"),
        assert: keyword("!assert"),
        if_: keyword("!if"),
        declare_return_function: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("!"),
            leaf(r"(unquoted\s|final\s)*"),
            leaf("function"),
            IRegex::space_one_or_more(),
            leaf(r"\$?"),
            leaf(IDENT),
        ]),
        declare_procedure: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("!"),
            leaf(r"(unquoted\s|final\s)*"),
            leaf("procedure"),
            IRegex::space_one_or_more(),
            leaf(r"\$?"),
            leaf(IDENT),
        ]),
        else_: keyword("!else"),
        elseif: keyword("!elseif"),
        endif: keyword("!endif"),
        while_: keyword("!while"),
        endwhile: keyword("!endwhile"),
        foreach: keyword("!foreach"),
        endfor: keyword("!endfor"),
        end_function: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("!end"),
            IRegex::space_zero_or_more(),
            leaf("(?:function|definelong|procedure)"),
            leaf(r"\b"),
        ]),
        return_: keyword("!return"),
        theme: keyword("!theme"),
        include: concat(vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            leaf("!include"),
            IRegex::space_zero_or_more(),
            leaf("(?:url|_many|_once)?"),
            leaf(r"\b"),
        ]),
        include_def: keyword("!includedef"),
        import: keyword("!import"),
        startsub: keyword("!startsub"),
        endsub: keyword("!endsub"),
        includesub: keyword("!includesub"),
        log: keyword("!log"),
        dump_memory: keyword("!dump_memory"),
        option: keyword("!option"),
    }
}

thread_local! {
    static P: Patterns = build_patterns();
}

impl TLineType {
    pub fn of(sl: &StringLocated) -> TLineType {
        P.with(|p| Self::of_with(sl, p))
    }

    fn of_with(sl: &StringLocated, p: &Patterns) -> TLineType {
        let s = sl.string();
        if p.comment_simple1.is_match(s) || p.comment_simple2.is_match(s) {
            return TLineType::CommentSimple;
        }
        if p.comment_long_start.is_match(s) && !s.contains("'/") {
            return TLineType::CommentLongStart;
        }
        if !s.contains('!') {
            return TLineType::Plain;
        }
        let table: [(&IRegex, TLineType); 30] = [
            (&p.legacy_define, TLineType::LegacyDefine),
            (&p.legacy_definelong, TLineType::LegacyDefineLong),
            (&p.affectation_define, TLineType::AffectationDefine),
            (&p.affectation, TLineType::Affectation),
            (&p.ifdef, TLineType::Ifdef),
            (&p.undef, TLineType::Undef),
            (&p.ifndef, TLineType::Ifndef),
            (&p.assert, TLineType::Assert),
            (&p.if_, TLineType::If),
            (&p.declare_return_function, TLineType::DeclareReturnFunction),
            (&p.declare_procedure, TLineType::DeclareProcedure),
            (&p.else_, TLineType::Else),
            (&p.elseif, TLineType::ElseIf),
            (&p.endif, TLineType::EndIf),
            (&p.while_, TLineType::While),
            (&p.endwhile, TLineType::EndWhile),
            (&p.foreach, TLineType::Foreach),
            (&p.endfor, TLineType::EndForeach),
            (&p.end_function, TLineType::EndFunction),
            (&p.return_, TLineType::Return),
            (&p.theme, TLineType::Theme),
            (&p.include, TLineType::Include),
            (&p.include_def, TLineType::IncludeDef),
            (&p.import, TLineType::Import),
            (&p.startsub, TLineType::StartSub),
            (&p.endsub, TLineType::EndSub),
            (&p.includesub, TLineType::IncludeSub),
            (&p.log, TLineType::Log),
            (&p.dump_memory, TLineType::DumpMemory),
            (&p.option, TLineType::Option),
        ];
        for (re, t) in table {
            if re.is_match(s) {
                return t;
            }
        }
        TLineType::Plain
    }

    pub fn is_quote(c: char) -> bool {
        c == '"' || c == '\''
    }

    /// `Character.isSpaceChar`.
    pub fn is_space_char(c: char) -> bool {
        matches!(
            c,
            ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
        )
    }

    pub fn is_latin_digit(c: char) -> bool {
        c.is_ascii_digit()
    }

    fn is_emoji(c: char) -> bool {
        c as u32 > 0xFFFF
    }

    fn is_letter_or_underscore(c: char) -> bool {
        c.is_alphabetic() || c == '_'
    }

    pub fn is_letter_or_emoji_or_underscore_or_digit(c: char) -> bool {
        Self::is_letter_or_underscore(c) || Self::is_latin_digit(c) || Self::is_emoji(c)
    }

    pub fn is_letter_or_emoji_or_underscore_or_dollar(c: char) -> bool {
        Self::is_letter_or_underscore(c) || c == '$' || Self::is_emoji(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> TLineType {
        TLineType::of(&StringLocated::new(s, None))
    }

    #[test]
    fn classifies() {
        assert_eq!(t("A -> B"), TLineType::Plain);
        assert_eq!(t("' x"), TLineType::CommentSimple);
        assert_eq!(t("/' x '/"), TLineType::CommentSimple);
        assert_eq!(t("/' x"), TLineType::CommentLongStart);
        assert_eq!(t("!$a = 1"), TLineType::Affectation);
        assert_eq!(t("!a ?= 1"), TLineType::Affectation);
        assert_eq!(t("!define X 1"), TLineType::AffectationDefine);
        assert_eq!(t("!define X(a) a"), TLineType::LegacyDefine);
        assert_eq!(t("!definelong X"), TLineType::LegacyDefineLong);
        assert_eq!(t("!if $a == 1"), TLineType::If);
        assert_eq!(t("!ifdef X"), TLineType::Ifdef);
        assert_eq!(t("!ifndef X"), TLineType::Ifndef);
        assert_eq!(t("!else"), TLineType::Else);
        assert_eq!(t("!elseif 1"), TLineType::ElseIf);
        assert_eq!(t("!endif"), TLineType::EndIf);
        assert_eq!(t("!while 1"), TLineType::While);
        assert_eq!(t("!endwhile"), TLineType::EndWhile);
        assert_eq!(t("!foreach $a in [1]"), TLineType::Foreach);
        assert_eq!(t("!endfor"), TLineType::EndForeach);
        assert_eq!(t("!function $f()"), TLineType::DeclareReturnFunction);
        assert_eq!(t("!unquoted procedure $p()"), TLineType::DeclareProcedure);
        assert_eq!(t("!endfunction"), TLineType::EndFunction);
        assert_eq!(t("!return 1"), TLineType::Return);
        assert_eq!(t("!theme x"), TLineType::Theme);
        assert_eq!(t("!include a.puml"), TLineType::Include);
        assert_eq!(t("!include_many a"), TLineType::Include);
        assert_eq!(t("!includesub a!B"), TLineType::IncludeSub);
        assert_eq!(t("!startsub A"), TLineType::StartSub);
        assert_eq!(t("!endsub"), TLineType::EndSub);
        assert_eq!(t("!log x"), TLineType::Log);
        assert_eq!(t("!option x 1"), TLineType::Option);
        assert_eq!(t("!pragma teoz true"), TLineType::Plain);
        assert!(TLineType::is_space_char(' ') && !TLineType::is_space_char('\t'));
    }
}
