use crate::render;

fn gantt(body: &str) -> String {
    render(&format!("@startgantt\n{body}\n@endgantt\n")).unwrap_or_else(|e| panic!("{e}: {body}"))
}

#[test]
fn tasks_and_dependencies() {
    let svg = gantt(
        "Project starts the 5th of december 2018\n\
         saturday are closed\nsunday are closed\n\
         2018/12/29 is opened\n2018/12/29 is colored in orange\n\
         [Prototype design] lasts 3 weeks\n\
         [Test prototype] starts at [Prototype design]'s end and lasts 2 days\n\
         [A]->[B]\n[Test prototype] is colored in red/blue\n\
         [Test prototype] is 40% completed",
    );
    assert!(svg.contains("data-diagram-type=\"GANTT\""));
    assert!(svg.contains("Prototype design"));
    assert!(svg.contains("December 2018"));
}

#[test]
fn milestones_resources_and_notes() {
    let svg = gantt(
        "Project starts 2020-07-01\n\
         [Task 1] on {Bob:50%} {Alice} lasts 10 days\n\
         note bottom\nhello note\nend note\n\
         [Done] happens at [Task 1]'s end\n\
         [Task 2] starts 3 days after [Task 1]'s start and lasts 2 weeks\n\
         [Task 2] displays on same row as [Task 1]\n\
         {Bob} is off on 2020-07-06\n{Alice} is off on 2020-07-08 to 2020-07-09\n\
         {Alice} is off on monday\n\
         [Task 1] pauses on 2020-07-03\n[Task 1] pauses on tuesday\n\
         [Task 3] as [T3] lasts 2 days and is colored in Lavender/LightBlue\n\
         then [Task 4] lasts 1 day\n\
         [T3] links to [[http://example.com]]",
    );
    assert!(svg.contains("hello note"));
    assert!(svg.contains("{Bob:50%}"));
    let hidden = gantt(
        "Project starts 2020-07-01\n[T] on {Bob} lasts 3 days\nhide resource names\nhide resource footbox",
    );
    assert!(!hidden.contains("{Bob}"));
}

#[test]
fn groups_separators_and_scales() {
    for scale in ["daily", "weekly", "monthly", "quarterly", "yearly"] {
        let svg = gantt(&format!(
            "printscale {scale}\nProject starts 2020-01-01\n-- first --\ngroup [G]\n[A] lasts 40 days\n[B] lasts 100 days\nend group\n--\n\
             separator just before 2020-02-01\nseparator just after 2020-03-01\nhide footbox\nshow footbox\n\
             2020-01-10 to 2020-01-12 are named [Holiday]\n2020-01-10 and 3 days are colored in lightblue\n\
             then 2 days are closed\nsunday are colored in #ccc\ntoday is 2020-02-02 and is colored in #AAF\n\
             label on first column and left aligned"
        ));
        assert!(svg.contains("<svg"), "{scale}");
    }
}

#[test]
fn closed_days_languages_and_hours() {
    let svg = gantt(
        "language fr\nProject starts 2024-09-22\nsaturday are closed\nsunday are closed\n\
         hide closed\nweek starts on monday with 4\n[T] starts 2024-09-23 and ends 2024-09-27\n\
         [U] lasts 2 days and 3 hours\nhide column start\nshow column start",
    );
    assert!(svg.contains("lundi") || svg.contains("lu"));
    let hours = gantt(
        "Project starts 2024-09-19\nFrom 8:00 to 12:00 are working hours\nFrom 14:00 to 18:00 are working hours\n\
         [Task 1] starts 2024-09-19 10:30:46 and ends 2024-09-24 10:30:47",
    );
    assert!(hours.contains("Task 1"));
    for lang in ["de", "es", "ja", "ko", "ru", "zh"] {
        let svg = gantt(&format!(
            "language {lang}\nProject starts 2020-07-01\n[A] lasts 2 days\n[B] lasts 21 days"
        ));
        assert!(svg.contains("<svg"));
    }
}

#[test]
fn relative_days_and_print_interval() {
    let svg = gantt("[T1] lasts 5 days\n[T2] starts 2 days after [T1]'s start and lasts 3 days\n[T1]-[bold,#red]->[T2]\nprintscale weekly");
    assert!(svg.contains("Day 1"));
    let svg = gantt(
        "Project starts 2020-07-01\nprint between 2020-07-05 and 2020-07-20\n[A] lasts 30 days\n[B] starts 2020-07-10 and lasts 3 days\n[C] starts 2020-07-01 and lasts 2 days\n[A]->[B]",
    );
    assert!(svg.contains("<svg"));
    gantt("task default completion to 50\nProject starts 2020-07-01\n[A] lasts 5 days");
    gantt("Project starts 2020-07-01\n[A] lasts 5 days\n[A] is deleted\n[B] lasts 2 days\n[B] is displayed as foo");
    gantt("Project starts 2020-07-01\n[M] happens 2020-07-03\n[M] is displayed as milestone");
}

#[test]
fn errors() {
    assert!(render("@startgantt\n[T] lasts foo\n@endgantt\n").is_err());
    assert!(
        render("@startgantt\n[A] lasts 3 days\nProject starts 2020-01-01\n@endgantt\n").is_err()
    );
    assert!(render("@startgantt\n[T] lasts 3 days\n[T] starts 2020-01-01\n@endgantt\n").is_err());
}
