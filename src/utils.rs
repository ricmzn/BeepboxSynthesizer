use anyhow::Context;
use godot::prelude::*;
use std::convert::Into;

pub(crate) trait V8ObjectHelpers {
    fn get<'a>(
        &self,
        scope: &mut v8::HandleScope<'a>,
        key: &str,
    ) -> anyhow::Result<v8::Local<'a, v8::Value>>;

    fn set<'a>(
        &self,
        scope: &mut v8::HandleScope<'a>,
        key: &str,
        value: impl Into<v8::Local<'a, v8::Value>>,
    ) -> anyhow::Result<()>;
}

impl<'a> V8ObjectHelpers for v8::Local<'a, v8::Object> {
    fn get<'scope>(
        &self,
        scope: &mut v8::HandleScope<'scope>,
        key: &str,
    ) -> anyhow::Result<v8::Local<'scope, v8::Value>> {
        let key = v8::String::new(scope, key).context("failed to create v8 string")?;
        let value =
            v8::Object::get(self, scope, key.into()).context("failed to get value from object")?;
        Ok(value)
    }

    fn set<'scope>(
        &self,
        scope: &mut v8::HandleScope<'scope>,
        key: &str,
        value: impl Into<v8::Local<'scope, v8::Value>>,
    ) -> anyhow::Result<()> {
        let key = v8::String::new(scope, key).context("failed to create v8 string")?;
        v8::Object::set(self, scope, key.into(), value.into())
            .context("failed to set value on object")?;
        Ok(())
    }
}

pub(crate) fn v8_value_to_godot_variant(
    scope: &mut v8::HandleScope,
    value: v8::Local<v8::Value>,
) -> Variant {
    match value {
        value if value.is_null_or_undefined() => Variant::nil(),
        value if value.is_number() => Variant::from(value.number_value(scope).unwrap()),
        value if value.is_boolean() => Variant::from(value.boolean_value(scope)),
        value if value.is_string() => Variant::from(value.to_rust_string_lossy(scope)),
        value if value.is_object() => {
            let dict = Dictionary::new();
            godot_warn!("TODO: js object -> dict conversion");
            Variant::from(dict)
        }
        value => unimplemented!(
            "value '{}' is of unsupported type",
            value.to_rust_string_lossy(scope)
        ),
    }
}
