use wasm_bindgen::prelude::*;

// Logging to typescript
#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn log_u32(a: u32);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn log_many(a: &str, b: &str);
}

#[macro_export]
macro_rules! decimal_ops {
    ($decimal:ident) => {
        ::paste::paste! {
            #[wasm_bindgen]
            pub fn [<get_ $decimal:snake _scale >] () -> BigInt {
                BigInt::from($decimal::scale())
            }

            #[wasm_bindgen]
            pub fn [<get_ $decimal:snake _denominator >] () -> BigInt {
                BigInt::from($decimal::from_integer(1).get())
            }

            #[wasm_bindgen]
            pub fn [<to_ $decimal:snake >] (js_val: u32, scale: u8) -> Result<BigInt,JsValue> {
                Ok(BigInt::from($decimal::from_scale(js_val, scale).get()))
            }
        }
    };
}
