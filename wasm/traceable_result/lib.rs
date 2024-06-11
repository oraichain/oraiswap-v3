#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::any;

pub type TrackableResult<T> = Result<T, TrackableError>;

#[derive(Debug)]
pub struct TrackableError {
    pub cause: String,
    pub stack: Vec<String>,
}

impl TrackableError {
    pub const ADD: &'static str = "addition overflow";
    pub const SUB: &'static str = "subtraction underflow";
    pub const MUL: &'static str = "multiplication overflow";
    pub const DIV: &'static str = "division overflow or division by zero";
    pub fn cast<T: ?Sized>() -> String {
        format!("conversion to {} type failed", any::type_name::<T>())
    }
}

impl TrackableError {
    pub fn new(cause: &str, location: &str) -> Self {
        Self {
            cause: cause.to_string(),
            stack: vec![location.to_string()],
        }
    }

    pub fn add_trace(&mut self, location: &str) {
        self.stack.push(location.to_string());
    }

    pub fn to_string(&self) -> String {
        let stack_trace = self.stack.join("\n-> ");

        format!(
            "ERROR CAUSED BY: {}\nINVARIANT STACK TRACE:\n-> {}",
            self.cause, stack_trace
        )
    }

    pub fn get(&self) -> (String, String, Vec<String>) {
        (
            self.to_string().clone(),
            self.cause.clone(),
            self.stack.clone(),
        )
    }
}

#[macro_use]
pub mod trackable_result {
    #[macro_export]
    macro_rules! from_result {
        ($op:expr) => {
            match $op {
                Ok(ok) => Ok(ok),
                Err(err) => Err(err!(&err)),
            }
        };
    }

    #[macro_export]
    macro_rules! err {
        ($error:expr) => {
            TrackableError::new($error, &location!())
        };
    }

    #[macro_export]
    macro_rules! ok_or_mark_trace {
        ($op:expr) => {
            match $op {
                Ok(ok) => Ok(ok),
                Err(mut err) => Err(trace!(err)),
            }
        };
    }

    #[macro_export]
    macro_rules! trace {
        ($deeper:expr) => {{
            $deeper.add_trace(&location!());
            $deeper
        }};
    }

    #[macro_export]
    macro_rules! function {
        () => {{
            fn f() {}
            fn type_name_of<T>(_: T) -> &'static str {
                core::any::type_name::<T>()
            }
            let name = type_name_of(f);
            &name[..name.len() - 3]
        }};
    }

    #[macro_export]
    macro_rules! location {
        () => {{
            alloc::format!("{}:{}:{}", file!(), function!(), line!())
        }};
    }

    #[macro_export]
    macro_rules! unwrap {
        ($res:expr) => {{
            $res.unwrap_or_else(|err| panic!("{:?}", err.get().0))
        }};
    }
}

#[cfg(test)]
mod trackable_error_tests {
    use super::*;

    fn value() -> TrackableResult<u64> {
        Ok(10u64)
    }

    fn inner_fun() -> TrackableResult<u64> {
        ok_or_mark_trace!(value())
    }

    fn outer_fun() -> TrackableResult<u64> {
        ok_or_mark_trace!(inner_fun())
    }

    fn trigger_error() -> TrackableResult<u64> {
        let _ = ok_or_mark_trace!(outer_fun())?; // unwrap without propagate error
        Err(err!("trigger error"))
    }

    fn trigger_result_error() -> Result<u64, String> {
        Err("trigger error [result])".to_string())
    }

    fn inner_fun_err() -> TrackableResult<u64> {
        ok_or_mark_trace!(trigger_error())
    }

    fn outer_fun_err() -> TrackableResult<u64> {
        ok_or_mark_trace!(inner_fun_err())
    }

    fn inner_fun_from_result() -> TrackableResult<u64> {
        from_result!(trigger_result_error())
    }

    fn outer_fun_from_result() -> TrackableResult<u64> {
        ok_or_mark_trace!(inner_fun_from_result())
    }

    #[test]
    fn test_trackable_result_type_flow() {
        // ok
        {
            let value = outer_fun().unwrap();
            assert_eq!(value, 10u64);
        }
        // error
        {
            let result = outer_fun_err();
            let err = result.unwrap_err();
            let (_, cause, stack) = err.get();

            assert_eq!(stack.len(), 3);
            assert_eq!(cause, "trigger error");
        }
        // from_result
        {
            let err = outer_fun_from_result().unwrap_err();
            let (_, cause, stack) = err.get();
            assert_eq!(stack.len(), 2);
            assert_eq!(cause, "trigger error [result])");
        }
    }

    #[test]
    fn test_unwrap() {
        let result = unwrap!(Result::Ok::<_, TrackableError>(0));
        assert_eq!(result, 0);
    }

    #[test]
    #[should_panic]
    fn test_unwrap_should_panic() {
        let _ = unwrap!(Result::Err(TrackableError::new("", "")));
    }
}
