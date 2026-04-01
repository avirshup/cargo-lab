/// Generate a struct named "--arg=value" style arguments
/// (at compile time) that can be passed through to another
/// program's CLI via its `cli_args` method.
#[macro_export]
macro_rules! build_passthrough_long_args {
    (
        $(#[$outer:meta])*
        $struct_name:ident {
        kv_flags: ($( $kvflag:ident ),* $(,)?),
        switch_flags: ($( $boolflag:ident ),* $(,)?),
    } ) => {
        $(#[$outer])*
        pub struct $struct_name {
            $(
                #[arg(long)] $kvflag: Option<String>,
            )*

            $(
                #[arg(long)] $boolflag: bool,
            )*
        }

        impl $struct_name {
            pub fn cli_args(&self) -> Vec<String> {
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
