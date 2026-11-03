use plantuml_core::render;

fn svg(body: &str) -> String {
    render(&format!("@startuml\n{body}\n@enduml")).unwrap()
}

fn count(svg: &str, tag: &str) -> usize {
    svg.matches(&format!("<{tag} ")).count()
}

#[test]
fn stop_after_endwhile_stands_beside_the_loop() {
    let plain = svg("start\nwhile (more?)\n:work;\nendwhile (no)\n:done;\nstop");
    let special = svg("start\nwhile (more?)\n:work;\nendwhile (no)\nstop");
    // the end marker sits left of the loop: two ellipses per marker, the stop one now beside the diamond
    assert_eq!(count(&plain, "ellipse"), count(&special, "ellipse"));
    let cx = |s: &str| -> Vec<f64> {
        s.split("<ellipse ")
            .skip(1)
            .map(|e| {
                e.split_once("cx=\"")
                    .unwrap()
                    .1
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap()
            })
            .collect()
    };
    let xs = cx(&special);
    assert!(xs.last().unwrap() < xs.first().unwrap(), "{xs:?}");
}

#[test]
fn kill_and_detach_end_the_flow_without_a_stop() {
    for word in ["kill", "detach"] {
        let s = svg(&format!(
            "start\nif (x?) then (yes)\n:a;\n{word}\nelse (no)\n:b;\nendif\n:c;\nstop"
        ));
        assert!(s.contains(">a<") && s.contains(">c<"), "{word}");
    }
}

#[test]
fn goto_and_label_render() {
    let s = svg("start\nlabel again\n:work;\nif (retry?) then (yes)\ngoto again\nendif\nstop");
    assert!(s.contains(">work<"));
}

#[test]
fn split_and_merge_render() {
    let s = svg("start\nsplit\n:a;\nsplit again\n:b;\nsplit again\n:c;\nend split\nstop");
    assert!(s.contains(">a<") && s.contains(">b<") && s.contains(">c<"));
    let m = svg("start\nfork\n:a;\nfork again\n:b;\nend merge\nstop");
    assert!(m.contains("<polygon"));
}

#[test]
fn notes_on_elseif_and_repeat_backward_render() {
    let s = svg("start\nif (a?) then (y)\n:x;\nelseif (b?) then (y)\n:z;\nnote right: hi\nelse (n)\n:w;\nendif\nstop");
    assert!(s.contains(">hi<"));
    let r = svg("start\nrepeat\n:a;\nnote right: hi\nbackward :b;\nrepeat while (c?)\nstop");
    assert!(r.contains(">hi<") && r.contains(">b<"));
}

#[test]
fn embedded_diagram_is_drawn_as_an_svg_image() {
    let s = svg(":Test\n{{\nnode N\n}}\n;");
    assert!(s.contains("data:image/svg+xml;base64,"), "{s}");
    assert!(!s.contains(">{{<"));
}
