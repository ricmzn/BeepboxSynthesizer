use godot::prelude::*;

#[repr(C)]
pub struct JSInspector {
    pub base: v8::inspector::V8InspectorClientBase,
}

impl v8::inspector::V8InspectorClientImpl for JSInspector {
    fn base(&self) -> &v8::inspector::V8InspectorClientBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut v8::inspector::V8InspectorClientBase {
        &mut self.base
    }

    unsafe fn base_ptr(this: *const Self) -> *const v8::inspector::V8InspectorClientBase
        where
            Self: Sized,
    {
        this.offset(0) as *const v8::inspector::V8InspectorClientBase
    }

    fn console_api_message(
        &mut self,
        _context_group_id: i32,
        _level: i32,
        message: &v8::inspector::StringView,
        url: &v8::inspector::StringView,
        line_number: u32,
        column_number: u32,
        _stack_trace: &mut v8::inspector::V8StackTrace,
    ) {
        godot_print!("{url}[{line_number}:{column_number}]: {message}");
    }
}
