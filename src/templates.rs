pub struct BuiltinTemplate {
    pub name: &'static str,
    pub content: &'static str,
}

pub const TEMPLATES: [BuiltinTemplate; 4] = [
    BuiltinTemplate {
        name: "bare",
        content: include_str!("../templates/bare.rs.template"),
    },
    BuiltinTemplate {
        name: "basic",
        content: include_str!("../templates/basic.rs.template"),
    },
    BuiltinTemplate {
        name: "clap",
        content: include_str!("../templates/clap.rs.template"),
    },
    BuiltinTemplate {
        name: "clap_subcmd",
        content: include_str!("../templates/clap_subcmd.rs.template"),
    },
];
