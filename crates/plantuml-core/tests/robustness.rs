//! Inputs that used to panic or overflow the stack must fail with an error or render.

fn render(src: &str) -> Result<String, plantuml_core::Error> {
    plantuml_core::render(src)
}

#[test]
fn recursive_function_is_an_error() {
    let src = "@startuml\n!function $f($x)\n!return $f($x)\n!endfunction\nA->B : $f(1)\n@enduml\n";
    assert!(render(src).is_err());
}

#[test]
fn recursive_procedure_is_an_error() {
    let src = "@startuml\n!procedure p()\np()\n!endprocedure\np()\n@enduml\n";
    assert!(render(src).is_err());
}

#[test]
fn sub_including_itself_is_an_error() {
    let src = "@startuml\n!startsub SUB\nAlice -> Bob : hello\n!includesub SUB\n!endsub\n@enduml\n";
    assert!(render(src).is_err());
}

#[test]
fn procedure_ending_inside_its_body_does_not_panic() {
    let src = "@startuml\n!startsub SUB\n!procedure box($desc)\n!endsub\n!endprocedure\ncard \"$desc\"\n\n!includesub SUB\n!startsub SUB\n\nbox(\"single line\")\n@enduml\n";
    let _ = render(src);
}

#[test]
fn teoz_par_without_end_does_not_panic() {
    let _ = render(
        r#"@startuml
!pragma teoz true
 [-> foo : test
& Bob -> Charlie
& par
     Alice -> Bob: Authentication Request
'& Charlie-> Bob: Authentication Request
&foo -> Alice : test

@enduml
"#,
    );
}

#[test]
fn teoz_delay_does_not_panic() {
    let _ = render(
        r#"@startuml

!pragma teoz true
a -> b: 1
& b -> c: 2
group test
... above ...
& b -> c: 4
... below ...
end
a -> b
& b -> c
a -> b
& b -> c
@enduml
' https://forum.plantuml.net/7710/teoz-and-%26-within-groups-is-not-working (linked from issue 2788)"#,
    );
}

#[test]
fn unbalanced_yaml_does_not_panic() {
    let _ = render(
        r#"@startyaml
!theme amiga
<style>
</style>
 -
    name: Mark McGwire
    hr:   65
    avg:  0.278
  -
{    name: Sammy Sosa
    hr:   63
    avg:  0.288

@endyaml
"#,
    );
}
