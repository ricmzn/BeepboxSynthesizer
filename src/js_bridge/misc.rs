pub mod performance {
    use std::time::Instant;

    use once_cell::sync::OnceCell;

    static START_TIME: OnceCell<Instant> = OnceCell::new();

    pub fn now(
        scope: &mut v8::HandleScope,
        _: v8::FunctionCallbackArguments,
        mut ret: v8::ReturnValue,
    ) {
        let time_diff = Instant::now().duration_since(unsafe { *START_TIME.get_unchecked() });
        let value = v8::Number::new(scope, time_diff.as_secs_f64() * 1000.0);
        ret.set(value.into());
    }

    pub fn start_performance_timer() {
        START_TIME.get_or_init(|| Instant::now());
    }
}
