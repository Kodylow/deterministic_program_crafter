extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// This macro generates a new function with the same signature as the input
/// function, but with the body replaced with a string representation of the
/// function.
///
/// The generated function can be used to print the function's source code or
/// other information.
///
/// # Example
///
/// ```
/// use my_proc_macro::function_to_string;
///
/// #[function_to_string]
/// fn my_function(x: i32, y: i32) -> i32 {
///     x + y
/// }
///
/// println!("{}", my_function(1, 2)); // Output: my_function(x: i32, y: i32) -> i32 { x + y }
/// ```
#[proc_macro_attribute]
pub fn function_to_tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input function
    let input_fn = parse_macro_input!(item as ItemFn);
    // Create string representation of the function
    let function_str = format!("{}", input_fn);

    // Define a new function with the same signature as the input function
    let fn_ident = input_fn.sig.ident.clone();
    let fn_inputs = input_fn.sig.inputs.clone();
    let fn_output = input_fn.sig.output.clone();
    let new_fn = quote! {
        #fn_ident #fn_inputs #fn_output {
            #output_fn
        }
    };

    // Generate output function
    let output_fn = quote! {
        pub fn #fn_ident #fn_generics (#fn_inputs) -> &'static str {
            #function_str
        }
    };

    // Return the new function
    output_fn.into()
}
