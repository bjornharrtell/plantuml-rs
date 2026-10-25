//! Notes: `note left of A`, `note over A, B`, `note across`, `note right` on a message
//! (`FactorySequenceNoteCommand`, `...OverSeveralCommand`, `...AcrossCommand`, `...OnArrowCommand`).

use super::model::{Note, NotePosition, NoteStyle};
use super::SequenceDiagram;
use crate::command::{CommandExecutionResult, MultiLineCommand, SingleLineCommand, Trim};
use crate::klimt::color::colors::{ColorType, Colors};
use crate::klimt::color::parser::ColorParser;
use crate::klimt::creole::display::Display;
use crate::regex::{IRegex, RegexResult};
use crate::stereo::{optional_pattern, Stereotype};
use crate::text::{BlocLines, LineLocation, MultilinesStrategy};
use crate::url::{self, UrlBuilder, UrlMode, URL_KEY};

type Cmd = Box<dyn crate::command::Command<SequenceDiagram>>;

const PARTICIPANT: &str = "([%pLN_.@]+|[%g][^%g]+[%g])";

fn color() -> ColorParser {
    ColorParser::simple_color(ColorType::Back)
}

fn strip_quotes(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s)
}

/// Pieces of a note command regex; `single` adds the `: text` tail.
struct Shape {
    parallel: bool,
    vmerge: bool,
    middle: Vec<IRegex>,
    /// Pattern of the last line of the multi-line form.
    end: &'static str,
}

impl Shape {
    fn regex(&self, single: bool) -> IRegex {
        let mut v = vec![IRegex::start()];
        if self.parallel {
            v.push(IRegex::named("PARALLEL", "(&[%s]*)?"));
        }
        if self.vmerge {
            v.push(IRegex::named("VMERGE", "(/)?"));
        }
        v.push(IRegex::space_zero_or_more());
        v.push(IRegex::named("STYLE", "(note|hnote|rnote)"));
        v.push(optional_pattern("STEREO1"));
        v.extend(self.middle.iter().cloned());
        v.push(optional_pattern("STEREO2"));
        v.push(color().regex());
        v.push(IRegex::space_zero_or_more());
        v.push(url::optional());
        if single {
            v.push(IRegex::space_zero_or_more());
            v.push(IRegex::leaf(":"));
            v.push(IRegex::space_zero_or_more());
            v.push(IRegex::named("NOTE", "(.*)"));
        }
        v.push(IRegex::end());
        IRegex::concat(v)
    }
}

type Exec = std::rc::Rc<
    dyn Fn(
        &mut SequenceDiagram,
        Option<LineLocation>,
        &RegexResult,
        Display,
    ) -> CommandExecutionResult,
>;

fn pair(multi_shape: &Shape, single_shape: &Shape, exec: Exec) -> Vec<Cmd> {
    let single_re = single_shape.regex(true);
    let multi_re = multi_shape.regex(false);
    let exec1 = exec.clone();
    let single: Cmd = Box::new(SingleLineCommand::new(
        single_re,
        move |d: &mut SequenceDiagram, loc, arg, _| {
            let display = Display::with_newlines(arg.get("NOTE", 0));
            let display = d.manage_variable(&display);
            exec1(d, loc, arg, display)
        },
    ));
    let starting = multi_re.clone();
    let multi: Cmd = Box::new(MultiLineCommand::new(
        multi_re,
        MultilinesStrategy::KeepStartingQuote,
        Trim::Both,
        multi_shape.end,
        move |d: &mut SequenceDiagram, lines: &BlocLines, _| {
            let first = lines.first().expect("first").trimmed();
            let arg = starting.matcher(first.string()).expect("starts");
            let body = lines.sub_extract(1, 1);
            let body = body.remove_empty_columns();
            let display = Display::create(&body.as_strings());
            let display = d.manage_variable(&display);
            exec(d, lines.location(), &arg, display)
        },
    ));
    vec![multi, single]
}

fn note_colors(
    d: &SequenceDiagram,
    arg: &RegexResult,
    note: &mut Note,
) -> Result<(), CommandExecutionResult> {
    let Some(mut colors) = color().colors(arg) else {
        return Err(CommandExecutionResult::bad_color());
    };
    if let Some(st) = arg.get_lazzy("STEREO", 0) {
        let stereotype = Stereotype::build(st);
        match d.base.skin_param.colors_for_note(&colors, &stereotype) {
            Some(c) => colors = c,
            None => return Err(CommandExecutionResult::bad_color()),
        }
        note.stereotype = Some(stereotype);
    }
    note.colors = colors;
    note.note_style = NoteStyle::parse(arg.get("STYLE", 0).unwrap_or("note"));
    if let Some(u) = arg.get(URL_KEY, 0) {
        let builder = UrlBuilder::new(d.base.skin_param.get_value("topurl"), UrlMode::Strict);
        note.url = builder.url(u);
    }
    Ok(())
}

fn finish_note(
    d: &mut SequenceDiagram,
    arg: &RegexResult,
    mut note: Note,
    allow_parallel: bool,
) -> CommandExecutionResult {
    if let Err(e) = note_colors(d, arg, &mut note) {
        return e;
    }
    let try_merge = arg.get("VMERGE", 0).is_some();
    if allow_parallel && arg.get("PARALLEL", 0).is_some() {
        note.parallel = true;
    }
    if try_merge {
        d.base.add_warning(vec![
            "The leading '/' note-merge syntax is deprecated; use '&' to place notes on the same row instead."
                .to_string(),
        ]);
    }
    d.add_note(note, try_merge);
    CommandExecutionResult::ok()
}

/// `note left of A`, `note over A` (`FactorySequenceNoteCommand`).
fn note_on_participant() -> Vec<Cmd> {
    let exec: Exec = std::rc::Rc::new(|d, loc, arg, display| {
        let p = d.get_or_create_participant(
            loc,
            strip_quotes(arg.get("PARTICIPANT", 0).expect("participant")),
            None,
        );
        let position =
            NotePosition::parse(arg.get("POSITION", 0).expect("position")).expect("note position");
        if display.size() == 0 {
            return CommandExecutionResult::ok();
        }
        let note = Note::new(Some(p), None, position, display, d.style_builder());
        finish_note(d, arg, note, true)
    });
    let shape = |space: &str| Shape {
        parallel: true,
        vmerge: true,
        middle: vec![
            IRegex::named("POSITION", "(right|left|over)"),
            IRegex::space_one_or_more(),
            IRegex::named("PARTICIPANT", &format!("(?:of[%s]{space})?{PARTICIPANT}")),
        ],
        end: "^end[%s]?(note|hnote|rnote)$",
    };
    let multi_shape = shape("+");
    let single_shape = shape("");
    pair(&multi_shape, &single_shape, exec)
}

/// `note over A, B` (`FactorySequenceNoteOverSeveralCommand`).
fn note_over_several() -> Vec<Cmd> {
    let exec: Exec = std::rc::Rc::new(|d, loc, arg, display| {
        let p1 =
            d.get_or_create_participant(loc, strip_quotes(arg.get("P1", 0).expect("p1")), None);
        let p2 =
            d.get_or_create_participant(loc, strip_quotes(arg.get("P2", 0).expect("p2")), None);
        if display.size() == 0 {
            return CommandExecutionResult::ok();
        }
        let note = Note::new(
            Some(p1),
            Some(p2),
            NotePosition::OverSeveral,
            display,
            d.style_builder(),
        );
        finish_note(d, arg, note, true)
    });
    let shape = Shape {
        parallel: true,
        vmerge: true,
        middle: vec![
            IRegex::leaf("over"),
            IRegex::space_one_or_more(),
            IRegex::named("P1", PARTICIPANT),
            IRegex::space_zero_or_more(),
            IRegex::leaf(","),
            IRegex::space_zero_or_more(),
            IRegex::named("P2", PARTICIPANT),
        ],
        end: "^end[%s]?(note|hnote|rnote)$",
    };
    pair(&shape, &shape, exec)
}

/// `note across` (`FactorySequenceNoteAcrossCommand`).
fn note_across() -> Vec<Cmd> {
    let exec: Exec = std::rc::Rc::new(|d, _, arg, display| {
        if arg
            .get("ACROSS", 0)
            .is_some_and(|a| a.eq_ignore_ascii_case("accross"))
        {
            return CommandExecutionResult::error("Use 'across' instead of 'accross'");
        }
        if display.size() == 0 {
            return CommandExecutionResult::ok();
        }
        let note = Note::new(
            None,
            None,
            NotePosition::OverSeveral,
            display,
            d.style_builder(),
        );
        finish_note(d, arg, note, false)
    });
    let shape = Shape {
        parallel: false,
        vmerge: true,
        middle: vec![IRegex::named("ACROSS", "(accross|across)")],
        end: "^end[%s]?(note|hnote|rnote)$",
    };
    pair(&shape, &shape, exec)
}

/// `note right` on the latest message (`FactorySequenceNoteOnArrowCommand`).
fn note_on_arrow() -> Vec<Cmd> {
    let exec: Exec = std::rc::Rc::new(|d, _, arg, display| {
        let Some(event) = d.last_event_with_note() else {
            return CommandExecutionResult::ok();
        };
        let position =
            NotePosition::parse(arg.get("POSITION", 0).expect("position")).expect("note position");
        let mut note = Note::new(None, None, position, display, d.style_builder());
        let Some(back) = color().colors(arg) else {
            return CommandExecutionResult::bad_color();
        };
        let mut colors = Colors::empty();
        if let Some(c) = back.color(ColorType::Back) {
            colors = colors.add(ColorType::Back, Some(c.clone()));
        }
        if let Err(e) = note_colors(d, arg, &mut note) {
            return e;
        }
        if note.stereotype.is_none() {
            note.colors = colors;
        }
        if let Some(w) = d.add_note_on_event(event, note) {
            d.base.add_warning(vec![w]);
        }
        CommandExecutionResult::ok()
    });
    let shape = Shape {
        parallel: false,
        vmerge: false,
        middle: vec![IRegex::named("POSITION", "(right|left|bottom|top)")],
        end: "^[%s]*end[%s]?note$",
    };
    pair(&shape, &shape, exec)
}

pub fn note_commands() -> Vec<Cmd> {
    let mut cmds = Vec::new();
    cmds.extend(note_on_arrow());
    cmds.extend(note_on_participant());
    cmds.extend(note_over_several());
    cmds.extend(note_across());
    cmds
}

#[cfg(test)]
mod tests {
    use crate::command::{create_system, UmlSource};
    use crate::sequencediagram::commands::commands;
    use crate::sequencediagram::model::{Event, NotePosition, NoteStyle};
    use crate::sequencediagram::SequenceDiagram;

    fn parse(text: &str) -> SequenceDiagram {
        let source = UmlSource::from_text(text);
        create_system(
            &commands(),
            SequenceDiagram::from_source(source.clone()),
            &source,
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn parses_note_commands() {
        let d = parse(
            "@startuml\nA -> B\nnote left of A : left\n/ hnote over B #red\nbody\nend hnote\n\
             rnote over A, B : several\nnote across : all\n& note right of B: par\n@enduml",
        );
        let ev = d.events();
        let Event::Notes(notes) = &ev[1] else {
            panic!("{:?}", ev[1])
        };
        assert_eq!(notes[0].position, NotePosition::Left);
        assert_eq!(notes[1].note_style, NoteStyle::Hexagonal);
        assert!(notes[1]
            .colors
            .color(crate::klimt::color::colors::ColorType::Back)
            .is_some());
        assert!(matches!(&ev[2], Event::Note(n) if n.position == NotePosition::OverSeveral));
        assert!(matches!(&ev[4], Event::Note(n) if n.parallel));
        assert_eq!(d.base.warnings.len(), 1);
    }

    #[test]
    fn note_on_message_and_errors() {
        let d = parse("@startuml\nA -> B\nnote right : on msg\nnote over A : x\n@enduml");
        let Event::Message(m) = &d.events()[0] else {
            panic!()
        };
        assert_eq!(m.core.notes.len(), 1);
        let source = UmlSource::from_text("@startuml\nnote over A #nosuchcolor: x\n@enduml");
        assert!(create_system(
            &commands(),
            SequenceDiagram::from_source(source.clone()),
            &source
        )
        .is_err());
        let source = UmlSource::from_text("@startuml\nnote accross : x\n@enduml");
        assert!(create_system(
            &commands(),
            SequenceDiagram::from_source(source.clone()),
            &source
        )
        .is_err());
    }
}
