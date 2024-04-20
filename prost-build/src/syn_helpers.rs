use syn::parse::Parse;

pub trait SynHelpers: AsRef<str> {
    /// Used internally for syn parsing where any errors are allowed to be immediatedly unwrapped.
    fn parse_syn<T: Parse>(&self) -> T {
        let input_str = self.as_ref();
        #[allow(clippy::expect_fun_call)]
        syn::parse_str(input_str).expect(&build_error_string::<T>(input_str))
    }
}

impl<T: AsRef<str>> SynHelpers for T {}

fn build_error_string<T>(input_str: &str) -> String {
    format!(
        "unable to parse {} as {}",
        input_str,
        std::any::type_name::<T>()
    )
}
