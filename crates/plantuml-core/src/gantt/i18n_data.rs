//! Month and day names per language (`I18nTimeData`, generated from the Java source).

pub const DAY_OF_WEEK_LONG: &[(&str, [&str; 7])] = &[
    (
        "de",
        [
            "Montag",
            "Dienstag",
            "Mittwoch",
            "Donnerstag",
            "Freitag",
            "Samstag",
            "Sonntag",
        ],
    ),
    (
        "es",
        [
            "lunes",
            "martes",
            "miércoles",
            "jueves",
            "viernes",
            "sábado",
            "domingo",
        ],
    ),
    (
        "fr",
        [
            "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
        ],
    ),
    (
        "ja",
        [
            "月曜日",
            "火曜日",
            "水曜日",
            "木曜日",
            "金曜日",
            "土曜日",
            "日曜日",
        ],
    ),
    (
        "ko",
        [
            "월요일",
            "화요일",
            "수요일",
            "목요일",
            "금요일",
            "토요일",
            "일요일",
        ],
    ),
    (
        "ru",
        [
            "понедельник",
            "вторник",
            "среда",
            "четверг",
            "пятница",
            "суббота",
            "воскресенье",
        ],
    ),
    (
        "zh",
        [
            "星期一",
            "星期二",
            "星期三",
            "星期四",
            "星期五",
            "星期六",
            "星期日",
        ],
    ),
    (
        "en",
        [
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
        ],
    ),
];

pub const DAY_OF_WEEK_SHORT: &[(&str, [&str; 7])] = &[
    ("de", ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"]),
    ("es", ["lu", "ma", "mi", "ju", "vi", "sá", "do"]),
    ("fr", ["lu", "ma", "me", "je", "ve", "sa", "di"]),
    ("ja", ["月", "火", "水", "木", "金", "土", "日"]),
    ("ko", ["월", "화", "수", "목", "금", "토", "일"]),
    ("ru", ["пн", "вт", "ср", "чт", "пт", "сб", "вс"]),
    (
        "zh",
        ["周一", "周二", "周三", "周四", "周五", "周六", "周日"],
    ),
    ("en", ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]),
];

pub const MONTH_SHORT: &[(&str, [&str; 12])] = &[
    (
        "de",
        [
            "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
        ],
    ),
    (
        "es",
        [
            "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
        ],
    ),
    (
        "fr",
        [
            "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
            "nov.", "déc.",
        ],
    ),
    (
        "ja",
        [
            "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
        ],
    ),
    (
        "ko",
        [
            "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월", "12월",
        ],
    ),
    (
        "ru",
        [
            "янв.",
            "февр.",
            "март",
            "апр.",
            "май",
            "июнь",
            "июль",
            "авг.",
            "сент.",
            "окт.",
            "нояб.",
            "дек.",
        ],
    ),
    (
        "zh",
        [
            "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
        ],
    ),
    (
        "en",
        [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ],
    ),
];

pub const MONTH_LONG: &[(&str, [&str; 12])] = &[
    (
        "de",
        [
            "Januar",
            "Februar",
            "März",
            "April",
            "Mai",
            "Juni",
            "Juli",
            "August",
            "September",
            "Oktober",
            "November",
            "Dezember",
        ],
    ),
    (
        "es",
        [
            "enero",
            "febrero",
            "marzo",
            "abril",
            "mayo",
            "junio",
            "julio",
            "agosto",
            "septiembre",
            "octubre",
            "noviembre",
            "diciembre",
        ],
    ),
    (
        "fr",
        [
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ],
    ),
    (
        "ja",
        [
            "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
        ],
    ),
    (
        "ko",
        [
            "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월", "12월",
        ],
    ),
    (
        "ru",
        [
            "январь",
            "февраль",
            "март",
            "апрель",
            "май",
            "июнь",
            "июль",
            "август",
            "сентябрь",
            "октябрь",
            "ноябрь",
            "декабрь",
        ],
    ),
    (
        "zh",
        [
            "一月",
            "二月",
            "三月",
            "四月",
            "五月",
            "六月",
            "七月",
            "八月",
            "九月",
            "十月",
            "十一月",
            "十二月",
        ],
    ),
    (
        "en",
        [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ],
    ),
];
