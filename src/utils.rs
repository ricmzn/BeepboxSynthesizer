use anyhow::Context;
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
