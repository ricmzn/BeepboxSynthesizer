use godot::log::{godot_error, godot_print, godot_warn};
use itertools::Itertools;
use rquickjs::prelude::Rest;
use rquickjs::{Ctx, Value};

pub fn console_log<'a>(ctx: Ctx<'a>, args: Rest<Value<'a>>) {
    godot_print!("[js] {}", format_args(ctx, args));
}

pub fn console_warn<'a>(ctx: Ctx<'a>, args: Rest<Value<'a>>) {
    godot_warn!("[js] {}", format_args(ctx, args));
}

pub fn console_error<'a>(ctx: Ctx<'a>, args: Rest<Value<'a>>) {
    godot_error!("[js] {}", format_args(ctx, args));
}

fn format_args<'a>(ctx: Ctx<'a>, args: Rest<Value<'a>>) -> String {
    args.iter()
        .filter_map(|value| stringify(&ctx, value))
        .map(js_string_to_rust_string)
        .join(" ")
}

fn stringify<'a>(ctx: &Ctx<'a>, value: &Value<'a>) -> Option<rquickjs::String<'a>> {
    match value.as_string() {
        Some(string) => Some(string.clone()),
        None => ctx
            .json_stringify(value)
            .inspect_err(|err| godot_warn!("[js] error in JSON.stringify(): {err}"))
            .ok()
            .flatten(),
    }
}

fn js_string_to_rust_string(string: rquickjs::String) -> String {
    string
        .to_string()
        .unwrap_or_else(|err| format!("(err: could not convert string: {err})"))
}
