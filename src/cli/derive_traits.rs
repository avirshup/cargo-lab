/// Something that can generate some arguments for another program
pub trait GeneratesArgs {
    fn cli_args(&self) -> Vec<String>;
}

/// Generate a struct named "--arg=value" style arguments
/// (at compile time) that can be passed through to another
/// program's CLI via its `cli_args` method.
///
/// Example:
/// ```rust
/// build_passthrough_long_args! {
///     /// my docstring
///     #[some_attrs]
///     #[another_attr]
///     pub struct StructName {
///         kv_flags(kv1, kv2),
///         switch_flags(switch1, switch2)
///     }
/// }
/// ```
///
/// Expands to (roughly)
///
/// ```
/// #[doc = r" my docstring"]
/// #[some_attr]
/// #[another_attr]
/// pub struct StructName {
///     #[arg(long)]  kv1: Option<String>,
///     #[arg(long)]  kv2: Option<String>,
///     #[arg(long)]  switch1: bool,
///     #[arg(long)]  switch2: bool,
/// }
/// impl StructName {
///     pub fn cli_args(&self) -> Vec<String> {
///         let mut vec = Vec::new();
///         if let Some(ref kv1) = self.kv1 { vec.push(format!("--kv1={kv1}")) };
///         if let Some(ref kv2) = self.kv2 { vec.push(format!("--kv2={kv2}")) };
///         if self.switch1 { vec.push("--switch1".to_owned()) } };
///         if self.switch2 { vec.push("--switch2".to_owned()) } };
///         vec
///     }
/// }
///
/// ## Notes
/// Semantically this should probably be a derive macro, but it's easy
/// enough to write declaratively and would need a whole new crate
/// (or really 2 to make it testable) for the proc macro.
///
/// This could also just be done at runtime with an array of
/// strings and a build-style clap CLI, instead of a
/// macro generating derive structs)
#[macro_export]
macro_rules! build_passthrough_long_args {
    (
        $(#[$outer:meta])*
        $vis:vis struct $struct_name:ident {
        kv_flags($( $kvflag:ident ),*),
        switch_flags($( $boolflag:ident ),* $(,)?)$(,)?
    } ) => {
        $(#[$outer])*
        $vis struct $struct_name {
            $(
                #[arg(long)] $kvflag: Option<String>,
            )*

            $(
                #[arg(long)] $boolflag: bool,
            )*
        }

        impl GeneratesArgs for $struct_name {
            fn cli_args(&self) -> Vec<String> {
                let mut vec = Vec::new();

                $(
                    if let Some(ref $kvflag) = self.$kvflag {
                        vec.push(format!(concat!("--", stringify!($kvflag), "={}"), $kvflag));
                    }
                )*

                $(
                    if self.$boolflag { vec.push(concat!("--", stringify!($boolflag)).to_owned()); }
                )*

                vec
            }
        }
    };
}
