use godot::prelude::*;
use rquickjs::Value;

pub fn js_value_to_godot_variant(value: Value) -> Variant {
    match value {
        value if value.is_null() || value.is_undefined() => Variant::nil(),
        value if value.is_number() => Variant::from(value.as_number().unwrap()),
        value if value.is_bool() => Variant::from(value.as_bool().unwrap()),
        value if value.is_string() => {
            Variant::from(value.as_string().unwrap().to_string().unwrap())
        }
        value if value.is_object() => {
            let dict = Dictionary::new();
            godot_warn!("TODO: js object -> dict conversion");
            Variant::from(dict)
        }
        value => unimplemented!(
            "value '{}' is of unsupported type",
            value.into_string().unwrap().to_string().unwrap()
        ),
    }
}
