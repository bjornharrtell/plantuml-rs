//! Texts of the table of tasks in several languages (`GanttI18n`).

fn pick(
    lang: &str,
    table: [(&'static str, &'static str); 7],
    default: &'static str,
) -> &'static str {
    table
        .iter()
        .find(|(l, _)| *l == lang)
        .map_or(default, |(_, t)| t)
}

pub fn task(lang: &str) -> &'static str {
    pick(
        lang,
        [
            ("fr", "Tâche"),
            ("es", "Tarea"),
            ("de", "Aufgabe"),
            ("ja", "タスク"),
            ("ko", "작업"),
            ("ru", "Задача"),
            ("zh", "任务"),
        ],
        "Task",
    )
}

pub fn start(lang: &str) -> &'static str {
    pick(
        lang,
        [
            ("fr", "Début"),
            ("es", "Inicio"),
            ("de", "Start"),
            ("ja", "開始"),
            ("ko", "시작"),
            ("ru", "Начало"),
            ("zh", "开始"),
        ],
        "Start",
    )
}

pub fn end(lang: &str) -> &'static str {
    pick(
        lang,
        [
            ("fr", "Fin"),
            ("es", "Fin"),
            ("de", "Ende"),
            ("ja", "終了"),
            ("ko", "끝"),
            ("ru", "Окончание"),
            ("zh", "结束"),
        ],
        "End",
    )
}

pub fn duration(lang: &str) -> &'static str {
    pick(
        lang,
        [
            ("fr", "Durée"),
            ("es", "Duración"),
            ("de", "Dauer"),
            ("ja", "期間"),
            ("ko", "기간"),
            ("ru", "Длительность"),
            ("zh", "持续时间"),
        ],
        "Duration",
    )
}

pub fn day_number(lang: &str, n: i64) -> String {
    match lang {
        "fr" => format!("Jour {n}"),
        "es" => format!("Día {n}"),
        "de" => format!("Tag {n}"),
        "ja" => format!("{n}日目"),
        "ko" => format!("{n}일째"),
        "ru" => format!("День {n}"),
        "zh" => format!("第{n}天"),
        _ => format!("Day {n}"),
    }
}

struct Units {
    none: &'static str,
    separator: &'static str,
    suffix_no_space: bool,
    /// Day, hour, minute and second, each with one to three forms.
    labels: [&'static [&'static str]; 4],
}

fn units(lang: &str) -> Units {
    match lang {
        "fr" => Units {
            none: "aucune",
            separator: ", ",
            suffix_no_space: false,
            labels: [
                &["jour", "jours"],
                &["heure", "heures"],
                &["minute", "minutes"],
                &["seconde", "secondes"],
            ],
        },
        "es" => Units {
            none: "ninguna",
            separator: ", ",
            suffix_no_space: false,
            labels: [
                &["día", "días"],
                &["hora", "horas"],
                &["minuto", "minutos"],
                &["segundo", "segundos"],
            ],
        },
        "de" => Units {
            none: "keine",
            separator: ", ",
            suffix_no_space: false,
            labels: [
                &["Tag", "Tage"],
                &["Stunde", "Stunden"],
                &["Minute", "Minuten"],
                &["Sekunde", "Sekunden"],
            ],
        },
        "ja" => Units {
            none: "なし",
            separator: "",
            suffix_no_space: true,
            labels: [&["日"], &["時間"], &["分"], &["秒"]],
        },
        "ko" => Units {
            none: "없음",
            separator: ", ",
            suffix_no_space: true,
            labels: [&["일"], &["시간"], &["분"], &["초"]],
        },
        "ru" => Units {
            none: "нет",
            separator: ", ",
            suffix_no_space: false,
            labels: [
                &["день", "дней", "дня"],
                &["час", "часов", "часа"],
                &["минута", "минут", "минуты"],
                &["секунда", "секунд", "секунды"],
            ],
        },
        "zh" => Units {
            none: "无",
            separator: "",
            suffix_no_space: true,
            labels: [&["天"], &["小时"], &["分钟"], &["秒"]],
        },
        _ => Units {
            none: "none",
            separator: ", ",
            suffix_no_space: false,
            labels: [
                &["day", "days"],
                &["hour", "hours"],
                &["minute", "minutes"],
                &["second", "seconds"],
            ],
        },
    }
}

fn label(forms: &[&'static str], value: i64) -> &'static str {
    match forms.len() {
        1 => forms[0],
        3 => {
            let (mod100, mod10) = (value % 100, value % 10);
            if mod10 == 1 && mod100 != 11 {
                forms[0]
            } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
                forms[2]
            } else {
                forms[1]
            }
        }
        _ => {
            if value > 1 {
                forms[1]
            } else {
                forms[0]
            }
        }
    }
}

/// `GanttI18n.durationHumanReadable`.
pub fn duration_human_readable(lang: &str, seconds: i64) -> String {
    let u = units(lang);
    if seconds == 0 {
        return u.none.to_string();
    }
    let days = seconds / 86_400;
    let rest = seconds % 86_400;
    let (hours, rest) = (rest / 3600, rest % 3600);
    let (minutes, secs) = (rest / 60, rest % 60);
    let mut out = String::new();
    for (i, v) in [days, hours, minutes, secs].into_iter().enumerate() {
        if v <= 0 {
            continue;
        }
        if !out.is_empty() {
            out.push_str(u.separator);
        }
        out.push_str(&v.to_string());
        if !u.suffix_no_space {
            out.push(' ');
        }
        out.push_str(label(u.labels[i], v));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(duration_human_readable("en", 86_400), "1 day");
        assert_eq!(duration_human_readable("en", 11 * 86_400), "11 days");
        assert_eq!(
            duration_human_readable("en", 90_061),
            "1 day, 1 hour, 1 minute, 1 second"
        );
        assert_eq!(duration_human_readable("en", 0), "none");
        assert_eq!(duration_human_readable("ru", 2 * 86_400), "2 дня");
        assert_eq!(duration_human_readable("ru", 5 * 86_400), "5 дней");
        assert_eq!(duration_human_readable("ru", 21 * 86_400), "21 день");
        assert_eq!(duration_human_readable("ja", 3 * 86_400 + 3600), "3日1時間");
        assert_eq!(duration_human_readable("de", 2 * 86_400), "2 Tage");
    }

    #[test]
    fn words() {
        assert_eq!(task("de"), "Aufgabe");
        assert_eq!(start("xx"), "Start");
        assert_eq!(end("fr"), "Fin");
        assert_eq!(duration("zh"), "持续时间");
        assert_eq!(day_number("en", 3), "Day 3");
        assert_eq!(day_number("zh", 3), "第3天");
        assert_eq!(day_number("ko", 3), "3일째");
    }
}
