fn main() {
    let src = "@startuml\nAlice -> Bob: hi\nBob --> Alice: ok\n@enduml\n";
    for i in 0..5 {
        let t = std::time::Instant::now();
        let r = plantuml_core::render(src);
        println!("{i}: {:?} {}", t.elapsed(), r.is_ok());
    }
    let cls = "@startuml\nclass A {\n+x()\n}\nclass B\nA <|-- B\n@enduml\n";
    for i in 0..3 {
        let t = std::time::Instant::now();
        let r = plantuml_core::render(cls);
        println!("class {i}: {:?} {}", t.elapsed(), r.is_ok());
    }
}
