//! Built-in functions (`tim/builtin`).

use super::context::TContext;
use super::expr::TValue;
use super::functions::{FunctionsSet, TFunction, TFunctionSignature, TFunctionType};
use super::memory::{TMemory, TVariableScope};
use super::{EaterException, Result};
use crate::json::JsonValue;
use crate::klimt::color::HColor;
use crate::text::StringLocated;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

type Named = HashMap<String, TValue>;
type Body =
    dyn Fn(&mut TContext, &mut TMemory, &StringLocated, &[TValue], &Named) -> Result<TValue>;

/// A return function implemented in Rust.
struct Builtin {
    signature: TFunctionSignature,
    cover: fn(usize) -> bool,
    body: Box<Body>,
}

impl TFunction for Builtin {
    fn signature(&self) -> &TFunctionSignature {
        &self.signature
    }

    fn can_cover(&self, nb_arg: usize, _named: &HashSet<String>) -> bool {
        (self.cover)(nb_arg)
    }

    fn function_type(&self) -> TFunctionType {
        TFunctionType::ReturnFunction
    }

    fn execute_return(
        &self,
        cx: &mut TContext,
        mem: &mut TMemory,
        location: &StringLocated,
        args: &[TValue],
        named: &Named,
    ) -> Result<TValue> {
        (self.body)(cx, mem, location, args, named)
    }

    fn execute_procedure(
        &self,
        _: &mut TContext,
        _: &mut TMemory,
        _: &StringLocated,
        _: &[TValue],
        _: &Named,
    ) -> Result<()> {
        panic!("not a procedure")
    }

    fn is_unquoted(&self) -> bool {
        false
    }
}

/// `%invoke_procedure`.
struct InvokeProcedure {
    signature: TFunctionSignature,
}

impl TFunction for InvokeProcedure {
    fn signature(&self) -> &TFunctionSignature {
        &self.signature
    }

    fn can_cover(&self, nb_arg: usize, _: &HashSet<String>) -> bool {
        nb_arg > 0
    }

    fn function_type(&self) -> TFunctionType {
        TFunctionType::Procedure
    }

    fn execute_return(
        &self,
        _: &mut TContext,
        _: &mut TMemory,
        _: &StringLocated,
        _: &[TValue],
        _: &Named,
    ) -> Result<TValue> {
        panic!("not a return function")
    }

    fn execute_procedure(
        &self,
        cx: &mut TContext,
        mem: &mut TMemory,
        location: &StringLocated,
        args: &[TValue],
        named: &Named,
    ) -> Result<()> {
        let fname = args[0].to_string();
        let sub = &args[1..];
        let func = cx
            .function_smart(&TFunctionSignature::new(&fname, sub.len()))
            .ok_or_else(|| {
                EaterException::new(format!("Cannot find void function {fname}"), location)
            })?;
        func.execute_procedure(cx, mem, location, sub, named)
    }

    fn is_unquoted(&self) -> bool {
        false
    }
}

fn add(
    set: &mut FunctionsSet,
    name: &str,
    declared: usize,
    cover: fn(usize) -> bool,
    body: impl Fn(&mut TContext, &mut TMemory, &StringLocated, &[TValue], &Named) -> Result<TValue>
        + 'static,
) {
    set.add_function(Rc::new(Builtin {
        signature: TFunctionSignature::new(name, declared),
        cover,
        body: Box::new(body),
    }));
}

fn s(v: &TValue) -> String {
    v.to_string()
}

fn err(msg: &str, loc: &StringLocated) -> EaterException {
    EaterException::new(msg, loc)
}

fn color_of(v: &TValue, loc: &StringLocated) -> Result<HColor> {
    HColor::parse(&s(v)).ok_or_else(|| err("No such color", loc))
}

/// Java `String.substring` on UTF-16 indexes, approximated on chars.
fn char_len(x: &str) -> usize {
    x.encode_utf16().count()
}

fn logical_not_of(b: bool) -> TValue {
    TValue::from_bool(!b)
}

/// Formats a time like `SimpleDateFormat` for the common letters (UTC).
fn format_date(pattern: &str, millis: i64) -> String {
    let secs = millis.div_euclid(1000);
    let ms = millis.rem_euclid(1000);
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (h, m, sec) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // civil from days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    const MONTHS: [&str; 12] = [
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
    ];
    const DAYS: [&str; 7] = [
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
    ];
    let weekday = DAYS[days.rem_euclid(7) as usize];
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                out.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() && chars[i + 1] == '\'' {
                out.push('\'');
                i += 1;
            }
            i += 1;
            continue;
        }
        if !c.is_ascii_alphabetic() {
            out.push(c);
            i += 1;
            continue;
        }
        let mut n = 1;
        while i + n < chars.len() && chars[i + n] == c {
            n += 1;
        }
        let num = |v: i64| format!("{v:0width$}", width = n);
        match c {
            'y' | 'Y' => {
                if n == 2 {
                    out.push_str(&format!("{:02}", y.rem_euclid(100)));
                } else {
                    out.push_str(&num(y));
                }
            }
            'M' | 'L' => match n {
                1 | 2 => out.push_str(&num(mo)),
                3 => out.push_str(&MONTHS[(mo - 1) as usize][..3]),
                _ => out.push_str(MONTHS[(mo - 1) as usize]),
            },
            'd' => out.push_str(&num(d)),
            'H' => out.push_str(&num(h)),
            'h' => out.push_str(&num(if h % 12 == 0 { 12 } else { h % 12 })),
            'm' => out.push_str(&num(m)),
            's' => out.push_str(&num(sec)),
            'S' => out.push_str(&num(ms)),
            'E' => {
                if n >= 4 {
                    out.push_str(weekday);
                } else {
                    out.push_str(&weekday[..3]);
                }
            }
            'a' => out.push_str(if h < 12 { "AM" } else { "PM" }),
            'Z' => out.push_str("+0000"),
            'z' => out.push_str("UTC"),
            _ => (0..n).for_each(|_| out.push(c)),
        }
        i += n;
    }
    out
}

fn next_int(cx: &mut TContext, bound: i32) -> i32 {
    // xorshift64*; seeded lazily from the clock
    if cx.rng_state == 0 {
        cx.rng_state = (cx.resolver.now_millis() as u64) ^ 0x9E37_79B9_7F4A_7C15;
        if cx.rng_state == 0 {
            cx.rng_state = 0x9E37_79B9_7F4A_7C15;
        }
    }
    let mut x = cx.rng_state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    cx.rng_state = x;
    let r = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33;
    if bound <= 0 {
        0
    } else {
        (r % bound as u64) as i32
    }
}

/// The set of functions always available (`StandardFunctions`).
pub fn standard_functions() -> Rc<FunctionsSet> {
    let mut f = FunctionsSet::new(None);
    let set = &mut f;
    add(
        set,
        "%false",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::from_bool(false)),
    );
    add(
        set,
        "%true",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::from_bool(true)),
    );
    add(
        set,
        "%backslash",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::text("\u{e110}")),
    );
    add(
        set,
        "%boolval",
        1,
        |n| n == 1,
        |_, _, l, a, _| {
            let v = s(&a[0]).to_lowercase();
            match v.as_str() {
                "true" | "1" => Ok(TValue::from_bool(true)),
                "false" | "0" => Ok(TValue::from_bool(false)),
                _ => Err(err(&format!("Cannot convert {v} to boolean."), l)),
            }
        },
    );
    add(
        set,
        "%breakline",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::text("\u{e103}")),
    );
    add(
        set,
        "%call_user_func",
        1,
        |n| n > 0,
        |cx, mem, l, a, named| {
            let fname = s(&a[0]);
            let args = &a[1..];
            let func = cx
                .function_smart(&TFunctionSignature::new(&fname, args.len()))
                .ok_or_else(|| err(&format!("Cannot find void function {fname}"), l))?;
            func.execute_return(cx, mem, l, args, named)
        },
    );
    add(
        set,
        "%chr",
        1,
        |n| n == 1,
        |_, _, _, a, _| {
            Ok(TValue::text(
                u32::try_from(a[0].to_int())
                    .ok()
                    .and_then(char::from_u32)
                    .map_or_else(|| "\0".to_string(), |c| c.to_string()),
            ))
        },
    );
    add(
        set,
        "%darken",
        2,
        |n| n == 2,
        |_, _, l, a, _| {
            let c = color_of(&a[0], l)?;
            Ok(TValue::text(c.darken(a[1].to_int()).as_string()))
        },
    );
    add(
        set,
        "%date",
        3,
        |n| n <= 3,
        |cx, _, l, a, _| {
            let now = cx.resolver.now_millis();
            if a.is_empty() {
                return Ok(TValue::text(format_date(
                    "EEE MMM dd HH:mm:ss zzz yyyy",
                    now,
                )));
            }
            let millis = if a.len() >= 2 {
                1000 * i64::from(a[1].to_int())
            } else {
                now
            };
            if a.len() == 3 && s(&a[2]) != "UTC" && s(&a[2]) != "GMT" {
                return Err(err(&format!("Unknown time zone: {}", s(&a[2])), l));
            }
            Ok(TValue::text(format_date(&s(&a[0]), millis)))
        },
    );
    add(
        set,
        "%dec2hex",
        1,
        |n| n == 1,
        |_, _, _, a, _| Ok(TValue::text(format!("{:x}", a[0].to_int()))),
    );
    add(
        set,
        "%dirpath",
        0,
        |n| n == 0,
        |cx, _, _, _, _| Ok(TValue::text(cx.environment_value("dirpath").unwrap_or(""))),
    );
    add(
        set,
        "%dollar",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::text("$")),
    );
    add(
        set,
        "%eval",
        1,
        |n| n == 1,
        |cx, mem, _, a, _| {
            let mut e = super::eater::Eater::from_text(&s(&a[0]));
            let v = e.eat_expression(cx, mem)?;
            Ok(TValue::Int(v.to_int()))
        },
    );
    add(
        set,
        "%feature",
        1,
        |n| n == 1,
        |_, _, _, a, _| {
            let arg = s(&a[0]);
            Ok(TValue::Int(i32::from(
                arg.eq_ignore_ascii_case("style") || arg.eq_ignore_ascii_case("theme"),
            )))
        },
    );
    add(
        set,
        "%filedate",
        0,
        |n| n == 0,
        |cx, _, _, _, _| Ok(TValue::text(cx.environment_value("filedate").unwrap_or(""))),
    );
    add(
        set,
        "%file_exists",
        1,
        |n| n == 1,
        |cx, _, _, a, _| Ok(TValue::from_bool(cx.resolver.file_exists(&s(&a[0])))),
    );
    add(
        set,
        "%filename",
        0,
        |n| n == 0,
        |cx, _, _, _, _| Ok(TValue::text(cx.environment_value("filename").unwrap_or(""))),
    );
    add(
        set,
        "%filename_no_extension",
        0,
        |n| n == 0,
        |cx, _, _, _, _| {
            Ok(TValue::text(
                cx.environment_value("filenameNoExtension").unwrap_or(""),
            ))
        },
    );
    add(
        set,
        "%function_exists",
        1,
        |n| n == 1,
        |cx, _, _, a, _| Ok(TValue::from_bool(cx.does_function_exist(&s(&a[0])))),
    );
    add(
        set,
        "%get_all_stdlib",
        1,
        |n| n <= 1,
        |cx, _, _, a, _| {
            let names = cx.resolver.all_stdlib_names();
            if a.is_empty() {
                return Ok(TValue::Json(JsonValue::Array(
                    names.iter().map(|n| JsonValue::from_str(n)).collect(),
                )));
            }
            Ok(TValue::Json(JsonValue::Object(
                names
                    .iter()
                    .map(|n| {
                        (
                            n.clone(),
                            JsonValue::Object(vec![
                                ("name".to_string(), JsonValue::from_str(n)),
                                ("version".to_string(), JsonValue::from_str("")),
                                ("source".to_string(), JsonValue::from_str("")),
                            ]),
                        )
                    })
                    .collect(),
            )))
        },
    );
    add(
        set,
        "%get_all_theme",
        0,
        |n| n == 0,
        |cx, _, _, _, _| {
            let mut names = cx.resolver.all_theme_names();
            names.sort();
            Ok(TValue::Json(JsonValue::Array(
                names.iter().map(|n| JsonValue::from_str(n)).collect(),
            )))
        },
    );
    add(
        set,
        "%get_current_theme",
        0,
        |n| n == 0,
        |cx, _, _, _, _| Ok(TValue::Json(cx.theme_metadata.clone())),
    );
    add(
        set,
        "%get_json_keys",
        1,
        |n| n == 1,
        |_, _, l, a, _| {
            let Some(json) = a[0].to_json() else {
                return Err(err("Not JSON data", l));
            };
            match json {
                JsonValue::Object(o) => Ok(TValue::Json(JsonValue::Array(
                    o.iter().map(|(k, _)| JsonValue::from_str(k)).collect(),
                ))),
                JsonValue::Array(arr) => {
                    let mut r = Vec::new();
                    for item in arr {
                        if let JsonValue::Object(o) = item {
                            r.extend(o.iter().map(|(k, _)| JsonValue::from_str(k)));
                        }
                    }
                    Ok(TValue::Json(JsonValue::Array(r)))
                }
                _ => Err(err("Bad JSON type", l)),
            }
        },
    );
    add(
        set,
        "%get_json_type",
        1,
        |n| n == 1,
        |_, _, _, a, _| {
            let r = match &a[0] {
                TValue::Str(_) => "string",
                TValue::Int(_) => "number",
                TValue::Json(j) => {
                    if j.is_array() {
                        "array"
                    } else if j.is_object() {
                        "object"
                    } else if j.is_boolean() {
                        "boolean"
                    } else if j.is_number() {
                        "number"
                    } else if j.is_string() {
                        "string"
                    } else {
                        "json"
                    }
                }
            };
            Ok(TValue::text(r))
        },
    );
    add(
        set,
        "%get_stdlib",
        1,
        |n| n <= 2,
        |_, _, _, a, _| {
            Ok(if a.len() == 2 {
                TValue::text("")
            } else {
                TValue::Json(JsonValue::Object(Vec::new()))
            })
        },
    );
    add(
        set,
        "%get_variable_value",
        1,
        |n| n == 1,
        |_, mem, _, a, _| {
            Ok(mem
                .get_variable(&s(&a[0]))
                .unwrap_or_else(|| TValue::text("")))
        },
    );
    add(
        set,
        "%version",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::text(crate::VERSION)),
    );
    add(
        set,
        "%getenv",
        1,
        |n| n == 1,
        |cx, _, _, a, _| Ok(TValue::text(cx.resolver.env(&s(&a[0])).unwrap_or_default())),
    );
    add(
        set,
        "%hex2dec",
        1,
        |n| n == 1,
        |_, _, _, a, _| Ok(TValue::Int(i32::from_str_radix(&s(&a[0]), 16).unwrap_or(0))),
    );
    add(
        set,
        "%hsl_color",
        3,
        |n| n == 3 || n == 4,
        |_, _, _, a, _| {
            let c = crate::klimt::color::Rgba::from_hsl([
                a[0].to_int() as f32,
                a[1].to_int() as f32,
                a[2].to_int() as f32,
            ]);
            Ok(TValue::text(HColor::simple(c).as_string()))
        },
    );
    add(
        set,
        "%intval",
        1,
        |n| n == 1,
        |_, _, l, a, _| {
            let v = s(&a[0]);
            v.trim_start_matches('+')
                .parse::<i32>()
                .map(TValue::Int)
                .map_err(|_| err(&format!("Cannot convert {v} to integer."), l))
        },
    );
    set.add_function(Rc::new(InvokeProcedure {
        signature: TFunctionSignature::new("%invoke_procedure", 1),
    }));
    add(
        set,
        "%is_dark",
        1,
        |n| n == 1,
        |_, _, l, a, _| Ok(TValue::from_bool(color_of(&a[0], l)?.is_dark())),
    );
    add(
        set,
        "%is_light",
        1,
        |n| n == 1,
        |_, _, l, a, _| Ok(TValue::from_bool(!color_of(&a[0], l)?.is_dark())),
    );
    add(
        set,
        "%json_add",
        3,
        |n| n == 2 || n == 3,
        |_, _, l, a, _| {
            let Some(json) = a[0].to_json() else {
                return Err(err("Not JSON data", l));
            };
            let mut json = json.clone();
            match &json {
                JsonValue::Array(_) => {
                    json.push(a[1].to_json_value());
                    Ok(TValue::Json(json))
                }
                JsonValue::Object(_) => {
                    if a.len() < 3 {
                        return Err(err("Bad JSON type", l));
                    }
                    json.add(&s(&a[1]), a[2].to_json_value());
                    Ok(TValue::Json(json))
                }
                _ => Ok(a[0].clone()),
            }
        },
    );
    add(
        set,
        "%json_key_exists",
        2,
        |n| n == 2,
        |_, _, _, a, _| {
            let Some(json) = a[0].to_json() else {
                return Ok(TValue::from_bool(false));
            };
            if !json.is_object() {
                return Ok(TValue::from_bool(false));
            }
            let is_key = match &a[1] {
                TValue::Str(_) => true,
                TValue::Json(j) => j.is_string(),
                TValue::Int(_) => false,
            };
            Ok(TValue::from_bool(is_key && json.contains(&s(&a[1]))))
        },
    );
    add(
        set,
        "%json_merge",
        2,
        |n| n == 2,
        |_, _, l, a, _| {
            let (Some(j0), Some(j1)) = (a[0].to_json(), a[1].to_json()) else {
                return Err(err("Not JSON data", l));
            };
            let mut json0 = j0.clone();
            let mixed =
                (json0.is_array() && j1.is_object()) || (json0.is_object() && j1.is_array());
            let scalars =
                !json0.is_array() && !json0.is_object() && !j1.is_array() && !j1.is_object();
            if scalars || mixed {
                return Ok(a[0].clone());
            }
            match (&mut json0, j1) {
                (JsonValue::Array(x), JsonValue::Array(y)) => {
                    x.extend(y.iter().cloned());
                    Ok(TValue::Json(json0))
                }
                (JsonValue::Object(_), JsonValue::Object(_)) => {
                    json0.merge(j1);
                    Ok(TValue::Json(json0))
                }
                _ => Err(err("Bad JSON type", l)),
            }
        },
    );
    add(
        set,
        "%json_remove",
        2,
        |n| n == 2,
        |_, _, l, a, _| {
            let Some(json) = a[0].to_json() else {
                return Err(err("Not JSON data", l));
            };
            let mut json = json.clone();
            match &mut json {
                JsonValue::Array(arr) => {
                    if let TValue::Int(i) = &a[1] {
                        if *i >= 0 && (*i as usize) < arr.len() {
                            arr.remove(*i as usize);
                        }
                    }
                    Ok(TValue::Json(json))
                }
                JsonValue::Object(_) => {
                    json.remove(s(&a[1]).as_str());
                    Ok(TValue::Json(json))
                }
                _ => Ok(a[0].clone()),
            }
        },
    );
    add(
        set,
        "%json_set",
        3,
        |n| n == 2 || n == 3,
        |_, _, l, a, _| {
            let Some(json) = a[0].to_json() else {
                return Err(err("Not JSON data", l));
            };
            let mut json = json.clone();
            if !json.is_array() && !json.is_object() {
                return Ok(a[0].clone());
            }
            if a.len() == 2 && json.is_object() {
                let value = a[1].to_json_value();
                if value.is_object() {
                    json.deep_merge(&value);
                }
                return Ok(TValue::Json(json));
            }
            if a.len() == 3 {
                if let JsonValue::Array(arr) = &json {
                    if let TValue::Int(i) = &a[1] {
                        if *i >= 0 && (*i as usize) < arr.len() {
                            json.set(*i as usize, a[2].to_json_value());
                        }
                    }
                    return Ok(TValue::Json(json));
                }
                json.set(s(&a[1]).as_str(), a[2].to_json_value());
                return Ok(TValue::Json(json));
            }
            Err(err("Error on json_set: Too many arguments", l))
        },
    );
    add(
        set,
        "%left_align",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::text("\u{e101}")),
    );
    add(
        set,
        "%lighten",
        2,
        |n| n == 2,
        |_, _, l, a, _| {
            let c = color_of(&a[0], l)?;
            Ok(TValue::text(c.lighten(a[1].to_int()).as_string()))
        },
    );
    add(
        set,
        "%load_json",
        3,
        |n| (1..=3).contains(&n),
        |cx, _, l, a, _| {
            let path = s(&a[0]);
            let data = if path.starts_with('<') || path.starts_with('>') {
                cx.resolver.read_stdlib(&path[1..path.len() - 1])
            } else if path.starts_with("http://") || path.starts_with("https://") {
                cx.resolver.read_url(&path)
            } else {
                cx.resolver
                    .read_file(&path, cx.current_dir.as_deref())
                    .map(|f| f.content)
            };
            let text = match data {
                Some(d) if !d.is_empty() => d,
                _ => {
                    if a.len() > 1 {
                        s(&a[1])
                    } else {
                        "{}".to_string()
                    }
                }
            };
            JsonValue::parse(&text).map(TValue::Json).map_err(|e| {
                err(
                    &format!("JSON parse issue in source {path} on location {}", e.offset),
                    l,
                )
            })
        },
    );
    add(
        set,
        "%and",
        2,
        |n| n >= 2,
        |_, _, _, a, _| Ok(TValue::from_bool(a.iter().all(TValue::to_boolean))),
    );
    add(
        set,
        "%nand",
        2,
        |n| n >= 2,
        |_, _, _, a, _| Ok(logical_not_of(a.iter().all(TValue::to_boolean))),
    );
    add(
        set,
        "%nor",
        2,
        |n| n >= 2,
        |_, _, _, a, _| Ok(logical_not_of(a.iter().any(TValue::to_boolean))),
    );
    add(
        set,
        "%not",
        1,
        |n| n == 1,
        |_, _, _, a, _| Ok(logical_not_of(a[0].to_boolean())),
    );
    add(
        set,
        "%nxor",
        2,
        |n| n >= 2,
        |_, _, _, a, _| {
            Ok(logical_not_of(
                a.iter().filter(|v| v.to_boolean()).count() == 1,
            ))
        },
    );
    add(
        set,
        "%or",
        2,
        |n| n >= 2,
        |_, _, _, a, _| Ok(TValue::from_bool(a.iter().any(TValue::to_boolean))),
    );
    add(
        set,
        "%xor",
        2,
        |n| n >= 2,
        |_, _, _, a, _| {
            Ok(TValue::from_bool(
                a.iter().filter(|v| v.to_boolean()).count() == 1,
            ))
        },
    );
    add(
        set,
        "%lower",
        1,
        |n| n == 1,
        |_, _, _, a, _| Ok(TValue::text(s(&a[0]).to_lowercase())),
    );
    add(
        set,
        "%mod",
        2,
        |n| n == 2,
        |_, _, l, a, _| {
            let d = a[1].to_int();
            if d == 0 {
                return Err(err("Divide by zero", l));
            }
            Ok(TValue::Int(a[0].to_int().wrapping_rem(d)))
        },
    );
    add(
        set,
        "%newline",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::text("\u{e100}")),
    );
    add(
        set,
        "%n",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::text("\u{e100}")),
    );
    add(
        set,
        "%now",
        0,
        |n| n == 0,
        |cx, _, _, _, _| Ok(TValue::Int((cx.resolver.now_millis() / 1000) as i32)),
    );
    add(
        set,
        "%ord",
        1,
        |n| n == 1,
        |_, _, _, a, _| Ok(TValue::Int(s(&a[0]).chars().next().map_or(0, |c| c as i32))),
    );
    add(
        set,
        "%percent",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::text("%")),
    );
    add(
        set,
        "%random",
        2,
        |n| n <= 2,
        |cx, _, _, a, _| {
            Ok(TValue::Int(match a.len() {
                0 => next_int(cx, 2),
                1 => next_int(cx, a[0].to_int()),
                _ => next_int(cx, a[1].to_int() - a[0].to_int()) + a[0].to_int(),
            }))
        },
    );
    add(
        set,
        "%retrieve_procedure",
        1,
        |n| n > 0,
        |cx, mem, l, a, _| {
            let fname = s(&a[0]);
            let args = &a[1..];
            let func = cx
                .function_smart(&TFunctionSignature::new(&fname, args.len()))
                .ok_or_else(|| err(&format!("Cannot find procedure {fname}"), l))?;
            let n1 = cx.result_list.len();
            func.execute_procedure(cx, mem, l, args, &HashMap::new())?;
            Ok(TValue::text(cx.extract_from_result_list(n1)))
        },
    );
    add(
        set,
        "%reverse_color",
        1,
        |n| n == 1,
        |_, _, l, a, _| {
            let c = color_of(&a[0], l)?;
            Ok(TValue::text(c.reverse().as_string()))
        },
    );
    add(
        set,
        "%reverse_hsluv_color",
        1,
        |n| n == 1,
        |_, _, l, a, _| {
            let c = color_of(&a[0], l)?;
            Ok(TValue::text(c.reverse_hsluv().as_string()))
        },
    );
    add(
        set,
        "%right_align",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::text("\u{e102}")),
    );
    add(
        set,
        "%set_variable_value",
        2,
        |n| n == 2,
        |_, mem, l, a, _| {
            mem.put_variable(&s(&a[0]), a[1].clone(), Some(TVariableScope::Global), l)?;
            Ok(TValue::text(""))
        },
    );
    add(
        set,
        "%size",
        1,
        |n| n == 1,
        |_, _, _, a, _| {
            Ok(TValue::Int(match &a[0] {
                TValue::Int(_) => 0,
                TValue::Str(x) => char_len(x) as i32,
                TValue::Json(j) => super::memory::json_size(j) as i32,
            }))
        },
    );
    add(
        set,
        "%splitstr",
        3,
        |n| n == 2,
        |_, _, _, a, _| {
            let text = s(&a[0]);
            let seps: Vec<char> = s(&a[1]).chars().collect();
            Ok(TValue::Json(JsonValue::Array(
                text.split(|c| seps.contains(&c))
                    .filter(|p| !p.is_empty())
                    .map(JsonValue::from_str)
                    .collect(),
            )))
        },
    );
    add(
        set,
        "%splitstr_regex",
        2,
        |n| n == 2,
        |_, _, l, a, _| {
            let re =
                fancy_regex::Regex::new(&s(&a[1])).map_err(|_| err("Bad regular expression", l))?;
            let text = s(&a[0]);
            let mut parts = Vec::new();
            let mut last = 0;
            for m in re.find_iter(&text).flatten() {
                if m.end() == 0 {
                    continue;
                }
                parts.push(&text[last..m.start()]);
                last = m.end();
            }
            parts.push(&text[last..]);
            while parts.last().is_some_and(|p| p.is_empty()) {
                parts.pop();
            }
            Ok(TValue::Json(JsonValue::Array(
                parts.into_iter().map(JsonValue::from_str).collect(),
            )))
        },
    );
    add(
        set,
        "%str2json",
        1,
        |n| n == 1,
        |_, _, _, a, _| {
            Ok(JsonValue::parse(&s(&a[0]))
                .map(TValue::Json)
                .unwrap_or_else(|_| TValue::text("")))
        },
    );
    add(
        set,
        "%string",
        1,
        |n| n == 1,
        |_, _, _, a, _| Ok(TValue::text(s(&a[0]))),
    );
    add(
        set,
        "%strlen",
        1,
        |n| n == 1,
        |_, _, _, a, _| Ok(TValue::Int(char_len(&s(&a[0])) as i32)),
    );
    add(
        set,
        "%strpos",
        2,
        |n| n == 2,
        |_, _, _, a, _| {
            let full = s(&a[0]);
            Ok(TValue::Int(match full.find(&s(&a[1])) {
                Some(b) => char_len(&full[..b]) as i32,
                None => -1,
            }))
        },
    );
    add(
        set,
        "%substr",
        3,
        |n| n == 2 || n == 3,
        |_, _, _, a, _| {
            let full: Vec<char> = s(&a[0]).chars().collect();
            let pos = a[1].to_int();
            if pos < 0 {
                return Err(EaterException::new(
                    "Fatal parsing error",
                    &StringLocated::new("", None),
                ));
            }
            let pos = pos as usize;
            if pos >= full.len() {
                return Ok(TValue::text(""));
            }
            let mut result: Vec<char> = full[pos..].to_vec();
            if a.len() == 3 {
                let len = a[2].to_int();
                if len >= 0 && (len as usize) < result.len() {
                    result.truncate(len as usize);
                }
            }
            Ok(TValue::text(result.into_iter().collect::<String>()))
        },
    );
    add(
        set,
        "%tab",
        0,
        |n| n == 0,
        |_, _, _, _, _| Ok(TValue::text("\u{e111}")),
    );
    add(
        set,
        "%upper",
        1,
        |n| n == 1,
        |_, _, _, a, _| Ok(TValue::text(s(&a[0]).to_uppercase())),
    );
    add(
        set,
        "%variable_exists",
        1,
        |n| n == 1,
        |_, mem, _, a, _| Ok(TValue::from_bool(mem.get_variable(&s(&a[0])).is_some())),
    );
    add(
        set,
        "%xargs",
        0,
        |n| n == 1,
        |cx, _, _, _, _| Ok(TValue::text(cx.xargs().unwrap_or_default())),
    );
    Rc::new(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(format_date("yyyy-MM-dd HH:mm:ss", 0), "1970-01-01 00:00:00");
        assert_eq!(
            format_date("EEEE d MMMM yy 'x' h a", 86_400_000 * 365),
            "Friday 1 January 71 x 12 AM"
        );
    }
}
