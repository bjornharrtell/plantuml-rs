use super::tick::{DateFormat, Decimal, DeduceFormat, TimingFormat};

fn render(src: &str) -> String {
    crate::render(src).expect("renders")
}

#[test]
fn decimal_and_formats() {
    let d = Decimal::parse("12.50").unwrap();
    assert_eq!(d.to_plain_string(), "12.50");
    assert_eq!(d.to_i64(), 12);
    assert_eq!(
        d.add(&Decimal::parse("-0.5").unwrap()).to_plain_string(),
        "12.00"
    );
    assert_eq!(Decimal::parse("-0.05").unwrap().to_plain_string(), "-0.05");
    assert!(Decimal::parse("x").is_none());
    assert_eq!(TimingFormat::Hour.format_time(3725), "1:02:05");
    assert_eq!(TimingFormat::Date.format_time(86400 * 31), "02/01");
    let f = DateFormat::new("yyyy-MM-dd HH:mm EEE MMM").unwrap();
    assert_eq!(f.format(0), "1970-01-01 00:00 Thu Jan");
    assert!(DateFormat::new("q").is_none());
    let a = DeduceFormat::from("1.5").unwrap();
    let m = a.merge_with(DeduceFormat::from("10.25").as_ref());
    assert_eq!(m.format(3.0), "3.0");
    assert_eq!(super::tick::java_double(5.0), "5.0");
    assert_eq!(super::tick::java_double(1e8), "1.0E8");
}

#[test]
fn clock_and_binary() {
    let svg = render(
        "@startuml\nclock \"Clk\" as C with period 10 pulse 3 offset 2\nbinary \"B\" as B\n\
         @0\nB is low\n@+10\nB is high\nC -> B@10 : go\n@enduml",
    );
    assert!(svg.contains("data-diagram-type=\"TIMING\""));
    assert!(svg.contains(">Clk</text>") && svg.contains(">go</text>"));
}

#[test]
fn state_players() {
    let svg = render(
        "@startuml\nrobust \"R\" as R\nconcise \"C\" as C\nrectangle \"X\" as X #pink\n\
         R has Idle,Busy\nC has \"Long label\" as L\nanalog \"A\" between 0 and 10 as A\nA ticks every 5\n\
         @0\nR is Idle\nC is L\nX is {-}\nA is 3\n@10 as :t\nR is {Idle,Busy} : c\nC is {...}\nX is hello #red\nA is 8\n\
         @20\nR is {hidden}\nC is {?}\nX is \"two words\"\nnote top of C : hi\nnote bottom of R\nline\nend note\n\
         R@0 <-> @10 : span\nhighlight 0 to 10 #Gold : zone\nR is 7pixels\n@enduml",
    );
    let _ = svg;
}

#[test]
fn settings() {
    let svg = render(
        "@startuml\nmode compact\nhide footbox\nuse date format \"yyyy/MM\"\nscale 1d as 20 pixels\n\
         concise C\nC is a\n@2000/01/01\nC is b\n@2000/02/01\nC is c\n@10:00:00\nmanual time axis\n@enduml",
    );
    assert!(svg.contains("<svg"));
}
