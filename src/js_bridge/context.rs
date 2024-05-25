use anyhow::{anyhow, Context, Result};
use godot::prelude::*;

use crate::js_bridge::{
    audio::create_audio_context,
    inspector::JSInspector,
    misc::performance,
    util::v8_value_to_godot_variant,
};

pub struct JSContext {
    isolate: v8::OwnedIsolate,
    global: v8::Global<v8::Context>,
    inspector: JSInspector,
}

impl JSContext {
    pub fn new() -> Result<JSContext> {
        let mut isolate = v8::Isolate::new(Default::default());

        let global = {
            let mut scope = v8::HandleScope::new(&mut isolate);
            let global = v8::ObjectTemplate::new(&mut scope);

            global.set(
                v8::String::new(&mut scope, "AudioContext").unwrap().into(),
                v8::FunctionTemplate::new(&mut scope, create_audio_context).into(),
            );

            global.set(
                v8::String::new(&mut scope, "navigator").unwrap().into(),
                v8::ObjectTemplate::new(&mut scope).into(),
            );

            global.set(
                v8::String::new(&mut scope, "document").unwrap().into(),
                v8::ObjectTemplate::new(&mut scope).into(),
            );

            let performance = v8::ObjectTemplate::new(&mut scope);
            let performance_now = v8::FunctionTemplate::new(&mut scope, performance::now);
            performance.set(
                v8::String::new(&mut scope, "now").unwrap().into(),
                performance_now.into(),
            );
            global.set(
                v8::String::new(&mut scope, "performance").unwrap().into(),
                performance.into(),
            );

            let context = v8::Context::new_from_template(&mut scope, global);

            // Wrap the context in a global object so its lifetime is unbound
            v8::Global::new(&mut scope, context)
        };

        let inspector = JSInspector {
            base: v8::inspector::V8InspectorClientBase::new::<JSInspector>()
        };

        let mut context = JSContext {
            isolate,
            global,
            inspector,
        };

        // Bind the window object for compatibility with browser scripts
        context.run(concat!(module_path!(), "::JSContext::new"), "const window = this")?;

        Ok(context)
    }

    pub fn run(&mut self, filename: &str, src: &str) -> Result<Variant> {
        self.do_scoped(filename, &mut |scope| {
            // Build and run the script
            let src = v8::String::new(scope, src).context("could not build v8 string")?;
            let result = v8::Script::compile(scope, src, None)
                .context("failed to compile script")?
                .run(scope);
            if let Some(value) = result {
                Ok(v8_value_to_godot_variant(scope, value))
            } else {
                Ok(Variant::nil())
            }
        })
    }

    pub fn do_scoped<'scope, T>(
        &'scope mut self,
        filename: &str,
        callback: &mut dyn FnMut(&mut v8::HandleScope<'scope>) -> Result<T>,
    ) -> Result<T> {
        // "Raw" script scope
        let mut scope = v8::HandleScope::new(&mut self.isolate);
        let context = v8::Local::new(&mut scope, self.global.clone());

        // Create and bind an inspector for console logging
        let mut inspector =
            v8::inspector::V8Inspector::create(&mut scope, &mut self.inspector);
        inspector.context_created(
            context,
            1,
            v8::inspector::StringView::from(b"Inspector" as &[u8]),
            v8::inspector::StringView::empty(),
        );

        // Script scope with globals + error handling
        let mut scope = v8::ContextScope::new(&mut scope, context);
        let mut scope = v8::TryCatch::new(&mut scope);

        // Run user callback using the scope
        let script_result = callback(&mut scope);

        if scope.has_caught() {
            let message = scope.message().context("could not extract error message")?;
            return script_result.context(anyhow!(
                "{} ({filename}:{})",
                message.get(&mut scope).to_rust_string_lossy(&mut scope),
                message.get_line_number(&mut scope).unwrap_or(0),
            ));
        }

        script_result
    }
}
