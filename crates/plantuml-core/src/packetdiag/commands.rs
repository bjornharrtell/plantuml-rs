//! Commands of packet diagrams (`packetdiag/command`).

use super::PacketDiagram;
use crate::command::{Command, CommandExecutionResult, SingleLineCommand};
use crate::regex::IRegex;

type Cmd = Box<dyn Command<PacketDiagram>>;

fn single(
    parts: Vec<IRegex>,
    f: impl Fn(&mut PacketDiagram, &crate::regex::RegexResult) -> CommandExecutionResult + 'static,
) -> Cmd {
    Box::new(SingleLineCommand::new(
        IRegex::concat(parts),
        move |d: &mut PacketDiagram, _, arg, _| f(d, arg),
    ))
}

fn setting(name: &'static str, value_name: &'static str, pattern: &'static str) -> Vec<IRegex> {
    vec![
        IRegex::start(),
        IRegex::leaf(name),
        IRegex::space_zero_or_more(),
        IRegex::leaf("="),
        IRegex::space_zero_or_more(),
        IRegex::named(value_name, pattern),
        IRegex::space_zero_or_more(),
        IRegex::end(),
    ]
}

pub fn commands() -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    cmds.extend(crate::command::common::common_commands());
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::named("TYPE", "(packetdiag)?"),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\{?"),
            IRegex::end(),
        ],
        |_, _| CommandExecutionResult::ok(),
    ));
    cmds.push(single(
        setting("colwidth", "WIDTH", r"(\d{1,3});?"),
        |d, arg| {
            if let Some(w) = arg.get("WIDTH", 0) {
                match w.parse::<i32>() {
                    Ok(w) => d.set_col_width(w),
                    Err(_) => return CommandExecutionResult::error("Width must be an integer"),
                }
            }
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        setting("node_height", "HEIGHT", r"(\d{1,3});?"),
        |d, arg| match arg.get("HEIGHT", 0).unwrap_or("").parse::<i32>() {
            Ok(h) => {
                d.update_node_height(h);
                CommandExecutionResult::ok()
            }
            Err(_) => CommandExecutionResult::error("Height must be an integer"),
        },
    ));
    cmds.push(single(
        setting("scale_direction", "DIR", "(ltr|rtl);?"),
        |d, arg| {
            d.set_scale_direction(arg.get("DIR", 0).unwrap_or("LTR"));
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        setting("scale_interval", "INTERVAL", r"(\d+);?"),
        |d, arg| match arg.get("INTERVAL", 0).unwrap_or("").parse::<i32>() {
            Ok(v) => {
                d.update_scale_interval(v);
                CommandExecutionResult::ok()
            }
            Err(_) => CommandExecutionResult::error("Scale interval invalid"),
        },
    ));
    cmds.push(single(
        setting("same_height", "VALUE", "(true|false);?"),
        |d, arg| {
            d.set_same_height(
                arg.get("VALUE", 0)
                    .is_some_and(|v| v.eq_ignore_ascii_case("true")),
            );
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::or(vec![
                IRegex::named("RANGE_UNIT", r"(\d{1,7})(?::)?"),
                IRegex::named("RANGE", r"(\d{1,7})(?:-(\d{1,7}))(?::)?"),
                IRegex::leaf(r"\*"),
            ]),
            IRegex::named("DESC", r"\s+(.*?)"),
            IRegex::named("ATTRIBUTE", r"(?:\s*\[(.*?)\])?"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |d, arg| {
            let r1 = arg.get("RANGE_UNIT", 0);
            let r2 = arg.get("RANGE", 0);
            let r3 = arg.get("RANGE", 1);
            let desc = arg.get("DESC", 0).unwrap_or("");
            let attr = crate::nwdiag::to_set(arg.get("ATTRIBUTE", 0));
            let int_attr = |name: &str, default: i32| {
                attr.get(name)
                    .and_then(|v| v.parse::<i32>().ok())
                    .unwrap_or(default)
            };
            let length = int_attr("len", 1);
            let (start, end);
            if let Some(r1) = r1 {
                let Ok(s) = r1.parse::<i32>() else {
                    return CommandExecutionResult::error(format!("Invalid bit start {r1}"));
                };
                start = s;
                end = s + length - 1;
            } else if let (Some(r2), Some(r3)) = (r2, r3) {
                match (r2.parse::<i32>(), r3.parse::<i32>()) {
                    (Ok(s), Ok(e)) => {
                        start = s;
                        end = e;
                    }
                    _ => {
                        return CommandExecutionResult::error(format!(
                            "Invalid bit range {r2} and {r3}"
                        ))
                    }
                }
            } else {
                start = d.last_packet_end().map_or(0, |v| v + 1);
                end = start + length - 1;
            }
            d.add_packet_item_range(
                start,
                end,
                int_attr("height", 1),
                desc,
                int_attr("rotate", 0),
            );
            CommandExecutionResult::ok()
        },
    ));
    cmds.push(single(
        vec![
            IRegex::start(),
            IRegex::space_zero_or_more(),
            IRegex::leaf(r"\}?"),
            IRegex::space_zero_or_more(),
            IRegex::end(),
        ],
        |_, _| CommandExecutionResult::ok(),
    ));
    cmds
}
